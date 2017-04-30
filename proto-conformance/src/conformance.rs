/// Represents a single test case's input.  The testee should:
///
///   1. parse this proto (which should always succeed)
///   2. parse the protobuf or JSON payload in "payload" (which may fail)
///   3. if the parse succeeded, serialize the message in the requested format.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct ConformanceRequest {
    /// Which format should the testee serialize its message to?
    #[proto(enumeration, tag="3")]
    pub requested_output_format: WireFormat,
    /// The payload (whether protobuf of JSON) is always for a
    /// protobuf_test_messages.proto3.TestAllTypes proto (as defined in
    /// src/google/protobuf/proto3_test_messages.proto).
    ///
    /// TODO(haberman): if/when we expand the conformance tests to support proto2,
    /// we will want to include a field that lets the payload/response be a
    /// protobuf_test_messages.proto2.TestAllTypes message instead.
    #[proto(oneof, tag="1", tag="2")]
    pub payload: Option<conformance_request::Payload>,
}
pub mod conformance_request {
    /// The payload (whether protobuf of JSON) is always for a
    /// protobuf_test_messages.proto3.TestAllTypes proto (as defined in
    /// src/google/protobuf/proto3_test_messages.proto).
    ///
    /// TODO(haberman): if/when we expand the conformance tests to support proto2,
    /// we will want to include a field that lets the payload/response be a
    /// protobuf_test_messages.proto2.TestAllTypes message instead.
    #[derive(Clone, Debug, PartialEq, Oneof)]
    pub enum Payload {
        #[proto(tag="1")]
        ProtobufPayload(Vec<u8>),
        #[proto(tag="2")]
        JsonPayload(String),
    }
}
/// Represents a single test case's output.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct ConformanceResponse {
    #[proto(oneof, tag="1", tag="6", tag="2", tag="3", tag="4", tag="5")]
    pub result: Option<conformance_response::Result>,
}
pub mod conformance_response {
    #[derive(Clone, Debug, PartialEq, Oneof)]
    pub enum Result {
        /// This string should be set to indicate parsing failed.  The string can
        /// provide more information about the parse error if it is available.
        ///
        /// Setting this string does not necessarily mean the testee failed the
        /// test.  Some of the test cases are intentionally invalid input.
        #[proto(tag="1")]
        ParseError(String),
        /// If the input was successfully parsed but errors occurred when
        /// serializing it to the requested output format, set the error message in
        /// this field.
        #[proto(tag="6")]
        SerializeError(String),
        /// This should be set if some other error occurred.  This will always
        /// indicate that the test failed.  The string can provide more information
        /// about the failure.
        #[proto(tag="2")]
        RuntimeError(String),
        /// If the input was successfully parsed and the requested output was
        /// protobuf, serialize it to protobuf and set it in this field.
        #[proto(tag="3")]
        ProtobufPayload(Vec<u8>),
        /// If the input was successfully parsed and the requested output was JSON,
        /// serialize to JSON and set it in this field.
        #[proto(tag="4")]
        JsonPayload(String),
        /// For when the testee skipped the test, likely because a certain feature
        /// wasn't supported, like JSON input/output.
        #[proto(tag="5")]
        Skipped(String),
    }
}
// This defines the conformance testing protocol.  This protocol exists between
// the conformance test suite itself and the code being tested.  For each test,
// the suite will send a ConformanceRequest message and expect a
// ConformanceResponse message.
//
// You can either run the tests in two different ways:
//
//   1. in-process (using the interface in conformance_test.h).
//
//   2. as a sub-process communicating over a pipe.  Information about how to
//      do this is in conformance_test_runner.cc.
//
// Pros/cons of the two approaches:
//
//   - running as a sub-process is much simpler for languages other than C/C++.
//
//   - running as a sub-process may be more tricky in unusual environments like
//     iOS apps, where fork/stdin/stdout are not available.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]
pub enum WireFormat {
    Unspecified = 0,
    Protobuf = 1,
    Json = 2,
}
