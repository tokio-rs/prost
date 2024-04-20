use std::ascii;
use std::borrow::Cow;
use std::collections::HashMap;
use std::iter;

use itertools::{Either, Itertools};
use log::debug;
use multimap::MultiMap;
use proc_macro2::TokenStream;
use prost_types::field_descriptor_proto::{Label, Type};
use prost_types::{
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FieldOptions, FileDescriptorProto,
    OneofDescriptorProto, ServiceDescriptorProto, SourceCodeInfo,
};
use quote::{quote, ToTokens};
use syn::parse::Parser;
use syn::{Attribute, TypePath};

use crate::ast::{Comments, Method, Service};
use crate::extern_paths::ExternPaths;
use crate::ident::{strip_enum_prefix, to_snake, to_upper_camel};
use crate::message_graph::MessageGraph;
use crate::SynHelpers;
use crate::{Config, FullyQualifiedName};

mod c_escaping;
use c_escaping::unescape_c_escape_string;

mod enums;
mod messages;
mod services;

mod syntax;
use syntax::Syntax;

// IMPROVEMENT: would be nice to have this auto-generated
mod locations;
use locations::*;

pub struct CodeGenerator<'a> {
    config: &'a mut Config,
    package: String,
    type_path: Vec<String>,
    source_info: Option<SourceCodeInfo>,
    syntax: Syntax,
    message_graph: &'a MessageGraph,
    extern_paths: &'a ExternPaths,
    path: Vec<i32>,
    buf: &'a mut String,
}

impl<'a> CodeGenerator<'a> {
    fn new(
        config: &'a mut Config,
        message_graph: &'a MessageGraph,
        extern_paths: &'a ExternPaths,
        source_code_info: Option<SourceCodeInfo>,
        package: Option<String>,
        syntax: Option<String>,
        buf: &'a mut String,
    ) -> Self {
        let source_info = source_code_info.map(|mut s| {
            s.location.retain(|loc| {
                let len = loc.path.len();
                len > 0 && len % 2 == 0
            });
            s.location.sort_by(|a, b| a.path.cmp(&b.path));
            s
        });

        Self {
            config,
            package: package.unwrap_or_default(),
            type_path: Vec::new(),
            source_info,
            syntax: syntax.as_ref().map(String::as_str).into(),
            message_graph,
            extern_paths,
            path: Vec::new(),
            buf,
        }
    }

    pub fn generate(
        config: &mut Config,
        message_graph: &MessageGraph,
        extern_paths: &ExternPaths,
        file: FileDescriptorProto,
        buf: &mut String,
    ) {
        let mut code_gen = CodeGenerator::new(
            config,
            message_graph,
            extern_paths,
            file.source_code_info,
            file.package,
            file.syntax,
            buf,
        );

        debug!(
            "file: {:?}, package: {:?}",
            file.name.as_ref().unwrap(),
            code_gen.package
        );

        code_gen.push_messages(file.message_type);
        code_gen.push_enums(file.enum_type);
        code_gen.push_services(file.service);
    }

    fn should_skip_debug(&self, fq_message_name: &FullyQualifiedName) -> bool {
        self.config
            .skip_debug
            .get(fq_message_name.as_ref())
            .next()
            .is_some()
    }

    fn resolve_field_attributes(
        &self,
        fully_qualified_name: &FullyQualifiedName,
        field_name: &str,
    ) -> TokenStream {
        let fq_str = fully_qualified_name.as_ref();
        let field_attributes = self.config.field_attributes.get_field(fq_str, field_name);

        quote! {
            #(#(#field_attributes)*)*
        }
    }

    fn comments_from_location(&self) -> Option<Comments> {
        let source_info = self.source_info.as_ref()?;
        let idx = source_info
            .location
            .binary_search_by_key(&&self.path[..], |location| &location.path[..])
            .unwrap();
        Some(Comments::from_location(&source_info.location[idx]))
    }

    fn resolve_docs(
        &self,
        fq_name: &FullyQualifiedName,
        field_name: Option<&str>,
    ) -> Vec<Attribute> {
        let mut comment_string = String::new();
        let disable_comments = &self.config.disable_comments;
        let append_doc = match field_name {
            Some(field_name) => disable_comments.get_first_field(fq_name, field_name),
            None => disable_comments.get(fq_name.as_ref()).next(),
        }
        .is_none();

        if append_doc {
            if let Some(comments) = self.comments_from_location() {
                comments.append_with_indent(&mut comment_string);
            }
        }

        match comment_string.is_empty() {
            true => Vec::new(),
            false => Attribute::parse_outer
                .parse_str(&comment_string)
                .expect("unable to parse comment attribute"),
        }
    }

    fn resolve_ident(&self, pb_ident: &FullyQualifiedName) -> String {
        if let Some(proto_ident) = self.extern_paths.resolve_ident(pb_ident) {
            return proto_ident;
        }

        let mut local_path = self
            .package
            .split('.')
            .chain(self.type_path.iter().map(String::as_str))
            .peekable();

        // If no package is specified the start of the package name will be '.'
        // and split will return an empty string ("") which breaks resolution
        // The fix to this is to ignore the first item if it is empty.
        if local_path.peek().map_or(false, |s| s.is_empty()) {
            local_path.next();
        }

        let mut ident_path = pb_ident.path_iterator();
        let ident_type = ident_path.next_back().unwrap();
        let mut ident_path = ident_path.peekable();

        // Skip path elements in common.
        while local_path.peek().is_some() && local_path.peek() == ident_path.peek() {
            local_path.next();
            ident_path.next();
        }

        local_path
            .map(|_| "super".to_string())
            .chain(ident_path.map(to_snake))
            .chain(iter::once(to_upper_camel(ident_type)))
            .join("::")
    }

    fn resolve_prost_path(&self) -> &str {
        self.config.prost_path.as_deref().unwrap_or("::prost")
    }

    fn prost_type_path(&self, item: &str) -> TypePath {
        syn::parse_str(&format!("{}::{}", self.resolve_prost_path(), item))
            .expect("unable to parse prost type path")
    }
}
