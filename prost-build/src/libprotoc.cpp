#include <stdio.h>
#include <string>
#include <vector>

#include <google/protobuf/compiler/importer.h>
#include <google/protobuf/descriptor.h>
#include <google/protobuf/descriptor_database.h>
#include <google/protobuf/io/coded_stream.h>
#include <google/protobuf/io/tokenizer.h>
#include <google/protobuf/repeated_field.h>
#include <google/protobuf/stubs/stringpiece.h>

#include <google/protobuf/port_def.inc>



extern "C" {
    // Small and simple and FFI safe, unlike `StringPiece`
    struct Path {
        const char* path;
        size_t len;
    };

    typedef void* (*resize)(void*, size_t);
    typedef size_t (*capacity)(void*);

    // Simple buffer wrapper so that we can write directly to memory allocated
    // in Rust
    struct Buffer {
        resize      resize_buffer;
        capacity    buffer_capacity;
        void*       context;
    };
}

using google::protobuf::FileDescriptor;
using google::protobuf::FileDescriptorSet;
using google::protobuf::DescriptorPool;
using google::protobuf::MergedDescriptorDatabase;
using google::protobuf::Message;
using google::protobuf::FileDescriptorProto;
using google::protobuf::RepeatedPtrField;

using google::protobuf::compiler::DiskSourceTree;
using google::protobuf::compiler::SourceTreeDescriptorDatabase;

// Port of CommandLineInterface::ParseInputFiles
bool parse_input_files(
    const std::vector<std::string>& input_files,
    DescriptorPool* descriptor_pool,
    std::vector<const FileDescriptor*>* parsed_files
) {
    for (const auto& input : input_files) {
        // Import the file.
        const FileDescriptor* parsed_file = descriptor_pool->FindFileByName(input);
        if (parsed_file == nullptr) {
            return false;
        }
        parsed_files->push_back(parsed_file);
    }

    return true;
}

bool make_inputs_relative(std::vector<std::string>* inputs, DiskSourceTree* source_tree) {
    for (auto& input_file : *inputs) {
        std::string virtual_file, shadowing_disk_file;

        auto mapping = source_tree->DiskFileToVirtualFile(
            input_file,
            &virtual_file,
            &shadowing_disk_file);

        switch (mapping) {
            case DiskSourceTree::SUCCESS: {
                input_file = virtual_file;
                break;
            }
            case DiskSourceTree::SHADOWED: {
                fprintf(stderr, "%s: Input is shadowed by an include in \"%s\"."
                    "Either use the latter file as your input or reorder the"
                    "includes so that the former file's location comes first.\n",
                    input_file.c_str(), shadowing_disk_file.c_str()
                );
                return false;
            }
            case DiskSourceTree::CANNOT_OPEN: {
                auto error_str = source_tree->GetLastErrorMessage().empty()
                                  ? strerror(errno)
                                  : source_tree->GetLastErrorMessage();
                fprintf(stderr, "Could not map to virtual file: %s: %s\n", input_file.c_str(), error_str.c_str());
                return false;
            }
            case DiskSourceTree::NO_MAPPING: {
                // Try to interpret the path as a virtual path.
                std::string disk_file;
                if (source_tree->VirtualFileToDiskFile(input_file, &disk_file)) {
                    return true;
                } else {
                    // The input file path can't be mapped to any --proto_path and it also
                    // can't be interpreted as a virtual path.
                    fprintf(stderr, "%s: File does not reside within any include path.\n", input_file.c_str());
                    return false;
                }
            }
        }
    }

    return true;
}

void get_transitive_deps(
    const FileDescriptor* file,
    bool include_json_name,
    bool include_source_code_info,
    std::set<const FileDescriptor*>* already_seen,
    RepeatedPtrField<FileDescriptorProto>* output
) {
    if (!already_seen->insert(file).second) {
        return;
    }

    for (int i = 0; i < file->dependency_count(); i++) {
        get_transitive_deps(
            file->dependency(i),
            include_json_name,
            include_source_code_info,
            already_seen,
            output
        );
    }

    FileDescriptorProto* new_descriptor = output->Add();
    file->CopyTo(new_descriptor);
    if (include_json_name) {
        file->CopyJsonNameTo(new_descriptor);
    }
    if (include_source_code_info) {
        file->CopySourceCodeInfoTo(new_descriptor);
    }
}

class BufferOutput : public google::protobuf::io::ZeroCopyOutputStream {
public:
    Buffer* impl;
    void* buffer_base = nullptr;
    size_t len = 0;

    BufferOutput(Buffer* impl) : impl(impl) {}

    bool Next(void** data, int* size) final {
        size_t old_size = this->len;
        size_t cur_cap = this->impl->buffer_capacity(this->impl->context);

        size_t new_size;
        if (old_size < cur_cap) {
            new_size = cur_cap;
        } else {
            new_size = old_size * 2;
        }

        // Avoid integer overflow in returned '*size'.
        new_size = std::min(new_size, old_size + std::numeric_limits<int>::max());
        new_size = std::max(new_size, size_t(1024));
        this->buffer_base = this->impl->resize_buffer(this->impl->context, new_size);

        *data = ((uint8_t*)buffer_base + old_size);
        *size = new_size - old_size;
        this->len = new_size;

        return true;
    }

    void BackUp(int count) final {
        this->buffer_base = this->impl->resize_buffer(this->impl->context, this->len - count);
    }

    int64_t ByteCount() const final {
        return this->len;
    }
};

bool write_descriptor_set(const std::vector<const FileDescriptor*>& parsed_files, Buffer* output) {
    FileDescriptorSet file_set;

    std::set<const FileDescriptor*> already_seen;

    for (const auto& parsed : parsed_files) {
        get_transitive_deps(
            parsed,
            true,  // Include json_name
            true, // Include source info, prost requires this
            &already_seen,
            file_set.mutable_file()
        );
    }

    {
        BufferOutput zero_copy_stream(output);
        google::protobuf::io::CodedOutputStream coded_out(&zero_copy_stream);

        // Determinism is useful here because build outputs are sometimes checked
        // into version control.
        coded_out.SetSerializationDeterministic(true);
        if (!file_set.SerializeToCodedStream(&coded_out)) {
            return false;
        }
    }

    return true;
}

// A MultiFileErrorCollector that prints errors to stderr.
class ErrorPrinter
    : public google::protobuf::compiler::MultiFileErrorCollector
    , public google::protobuf::io::ErrorCollector
    , public DescriptorPool::ErrorCollector {
public:
    ErrorPrinter()
      : found_errors_(false),
        found_warnings_(false) {}
  ~ErrorPrinter() {}

    // implements MultiFileErrorCollector ------------------------------
    void AddError(  const std::string& filename, int line, int column,
                    const std::string& message) override {
        found_errors_ = true;
        AddErrorOrWarning(filename, line, column, message, "error", std::cerr);
    }

    void AddWarning(const std::string& filename, int line, int column,
                    const std::string& message) override {
        found_warnings_ = true;
        AddErrorOrWarning(filename, line, column, message, "warning", std::clog);
    }

    // implements io::ErrorCollector -----------------------------------
    void AddError(int line, int column, const std::string& message) final {
        AddError("input", line, column, message);
    }

    void AddWarning(int line, int column, const std::string& message) final {
        AddErrorOrWarning("input", line, column, message, "warning", std::clog);
    }

    // implements DescriptorPool::ErrorCollector-------------------------
    void AddError(  const std::string& filename, const std::string& element_name,
                    const Message* descriptor, ErrorLocation location,
                    const std::string& message) override {
        AddErrorOrWarning(filename, -1, -1, message, "error", std::cerr);
    }

    void AddWarning(const std::string& filename, const std::string& element_name,
                    const Message* descriptor, ErrorLocation location,
                    const std::string& message) final {
        AddErrorOrWarning(filename, -1, -1, message, "warning", std::clog);
    }

    bool FoundErrors() const { return found_errors_; }

    bool FoundWarnings() const { return found_warnings_; }

private:
    void AddErrorOrWarning( const std::string& filename, int line, int column,
                            const std::string& message, const std::string& type,
                            std::ostream& out) {
        out << filename;

        // Users typically expect 1-based line/column numbers, so we add 1
        // to each here.
        if (line != -1) {
            out << ":" << (line + 1) << ":" << (column + 1);
        }

        if (type == "warning") {
            out << ": warning: " << message << std::endl;
        } else {
            out << ": " << message << std::endl;
        }
    }

    bool found_errors_;
    bool found_warnings_;
};

extern "C" {
    int write_descriptor_set(
        const Path* input_files,
        size_t num_inputs,
        const Path* includes_paths,
        size_t num_includes,
        Buffer* output
    ) {
        // We're forced to reallocate these because some of the following APIs
        // only take std::string :p
        std::vector<std::string> inputs;
        inputs.reserve(num_inputs);
        // I'm sure there's some fancier way to initialize this these days but I
        // prefer to keep the C++ as dumb as possible
        for (size_t i = 0; i < num_inputs; ++i) {
            inputs.push_back(std::string(input_files[i].path, input_files[i].len));
        }

        std::vector<std::string> includes;
        includes.reserve(num_includes);
        for (size_t i = 0; i < num_includes; ++i) {
            includes.push_back(std::string(includes_paths[i].path, includes_paths[i].len));
        }

        // Port of CommandLineInterface::InitializeDiskSourceTree
        std::unique_ptr<DiskSourceTree> source_tree(new DiskSourceTree());
        std::unique_ptr<MergedDescriptorDatabase> descriptor_set_in_database;

        // Set up the source tree. Note that the way prost uses protoc, virtual
        // paths would only be possible if the user is explicitly sending their
        // own -I args with the ':/;' separated virtual path. I seriously doubt
        // this happens in practice. (famous last words)
        for (const auto& include : includes) {
            source_tree->MapPath(include, "");
        }

        // Map input files to virtual paths if possible. I'm not sure if this
        // is even needed since as stated prost really doesn't do virtual paths
        if (!make_inputs_relative(&inputs, source_tree.get())) {
            return 1;
        }

        std::unique_ptr<ErrorPrinter> error_collector(new ErrorPrinter());
        std::unique_ptr<SourceTreeDescriptorDatabase> source_tree_database(
            new SourceTreeDescriptorDatabase(source_tree.get(), descriptor_set_in_database.get()));
        source_tree_database->RecordErrorsTo(error_collector.get());
        std::unique_ptr<DescriptorPool> descriptor_pool(new DescriptorPool(
            source_tree_database.get(),
            source_tree_database->GetValidationErrorCollector()));
        descriptor_pool->EnforceWeakDependencies(true);

        // Try to actually parse all of our inputs, if this
        std::vector<const FileDescriptor*> parsed;
        parsed.reserve(inputs.size());
        if (!parse_input_files(inputs, descriptor_pool.get(), &parsed)) {
            return 1;
        }

        if (!write_descriptor_set(parsed, output)) {
            return 1;
        }

        return 0;
    }
}