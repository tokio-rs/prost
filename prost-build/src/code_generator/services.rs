use super::*;

impl CodeGenerator<'_> {
    pub(super) fn push_services(&mut self, services: Vec<ServiceDescriptorProto>) {
        if self.config.service_generator.is_some() {
            self.path.push(6);
            for (idx, service) in services.into_iter().enumerate() {
                self.path.push(idx as i32);
                self.push_service(service);
                self.path.pop();
            }

            if let Some(service_generator) = self.config.service_generator.as_mut() {
                service_generator.finalize(self.buf);
            }

            self.path.pop();
        }
    }

    fn push_service(&mut self, service: ServiceDescriptorProto) {
        let name = service.name().to_owned();
        debug!("  service: {:?}", name);

        let comments = self.comments_from_location().unwrap_or_default();

        self.path.push(2);
        let methods = service
            .method
            .into_iter()
            .enumerate()
            .map(|(idx, mut method)| {
                debug!("  method: {:?}", method.name());

                self.path.push(idx as i32);
                let comments = self.comments_from_location().unwrap_or_default();
                self.path.pop();

                let name = method.name.take().unwrap();
                let input_proto_type = method.input_type.take().unwrap();
                let output_proto_type = method.output_type.take().unwrap();
                let input_type =
                    self.resolve_ident(&FullyQualifiedName::from_type_name(&input_proto_type));
                let output_type =
                    self.resolve_ident(&FullyQualifiedName::from_type_name(&output_proto_type));
                let client_streaming = method.client_streaming();
                let server_streaming = method.server_streaming();

                Method {
                    name: to_snake(&name),
                    proto_name: name,
                    comments,
                    input_type,
                    output_type,
                    input_proto_type,
                    output_proto_type,
                    options: method.options.unwrap_or_default(),
                    client_streaming,
                    server_streaming,
                }
            })
            .collect();
        self.path.pop();

        let service = Service {
            name: to_upper_camel(&name),
            proto_name: name,
            package: self.package.clone(),
            comments,
            methods,
            options: service.options.unwrap_or_default(),
        };

        if let Some(service_generator) = self.config.service_generator.as_mut() {
            service_generator.generate(service, self.buf)
        }
    }
}
