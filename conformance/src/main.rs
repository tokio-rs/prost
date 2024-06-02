use std::io::{self, Read, Write};

use bytes::{Buf, BufMut};
use prost::Message;

use protobuf::conformance::{
    conformance_request, conformance_response, ConformanceRequest, ConformanceResponse,
    TestCategory, WireFormat,
};
use protobuf::test_messages::proto2::TestAllTypesProto2;
use protobuf::test_messages::proto3::TestAllTypesProto3;
use tests::{roundtrip2, RoundtripInput, RoundtripOutputType, RoundtripResult2};

fn main() -> io::Result<()> {
    env_logger::init();

    let mut registry = prost_types::any_v2::TypeRegistry::new_with_well_known_types();
    registry.insert_msg_type_for_type_url::<TestAllTypesProto2>(
        "type.googleapis.com/protobuf_test_messages.proto2.TestAllTypesProto2",
    );
    registry.insert_msg_type_for_type_url::<TestAllTypesProto3>(
        "type.googleapis.com/protobuf_test_messages.proto3.TestAllTypesProto3",
    );

    let type_resolver = registry.into_type_resolver();
    prost_types::any_v2::with_type_resolver(Some(type_resolver), entrypoint)
}

fn entrypoint() -> io::Result<()> {
    let mut bytes = vec![0; 4];

    loop {
        bytes.resize(4, 0);

        if io::stdin().read_exact(&mut bytes).is_err() {
            // No more test cases.
            return Ok(());
        }

        let len = bytes.as_slice().get_u32_le() as usize;

        bytes.resize(len, 0);
        io::stdin().read_exact(&mut bytes)?;

        let result = match ConformanceRequest::decode(bytes.as_slice()) {
            Ok(request) => handle_request(request),
            Err(error) => conformance_response::Result::ParseError(format!("{:?}", error)),
        };

        let response = ConformanceResponse {
            result: Some(result),
        };

        let len = response.encoded_len();
        bytes.clear();
        bytes.put_u32_le(len as u32);
        response.encode(&mut bytes)?;
        assert_eq!(len + 4, bytes.len());

        let mut stdout = io::stdout();
        stdout.lock().write_all(&bytes)?;
        stdout.flush()?;
    }
}

fn handle_request(request: ConformanceRequest) -> conformance_response::Result {
    let output_ty = match request.requested_output_format() {
        WireFormat::Unspecified => {
            return conformance_response::Result::ParseError(
                "output format unspecified".to_string(),
            );
        }
        WireFormat::Jspb => {
            return conformance_response::Result::Skipped(
                "JSPB output is not supported".to_string(),
            );
        }
        WireFormat::TextFormat => {
            return conformance_response::Result::Skipped(
                "TEXT_FORMAT output is not supported".to_string(),
            );
        }
        WireFormat::Protobuf => RoundtripOutputType::Protobuf,
        WireFormat::Json => RoundtripOutputType::Json,
    };

    let input = match &request.payload {
        None => return conformance_response::Result::ParseError("no payload".to_string()),

        Some(conformance_request::Payload::JspbPayload(_)) => {
            return conformance_response::Result::Skipped(
                "JSON input is not supported".to_string(),
            );
        }
        Some(conformance_request::Payload::TextPayload(_)) => {
            return conformance_response::Result::Skipped(
                "JSON input is not supported".to_string(),
            );
        }
        Some(conformance_request::Payload::ProtobufPayload(buf)) => RoundtripInput::Protobuf(buf),
        Some(conformance_request::Payload::JsonPayload(buf)) => RoundtripInput::Json(buf),
    };

    let ignore_unknown_fields =
        request.test_category() == TestCategory::JsonIgnoreUnknownParsingTest;

    let roundtrip = match &*request.message_type {
        "protobuf_test_messages.proto2.TestAllTypesProto2" => {
            roundtrip2::<TestAllTypesProto2>(input, output_ty, ignore_unknown_fields)
        }
        "protobuf_test_messages.proto3.TestAllTypesProto3" => {
            roundtrip2::<TestAllTypesProto3>(input, output_ty, ignore_unknown_fields)
        }
        _ => {
            return conformance_response::Result::ParseError(format!(
                "unknown message type: {}",
                request.message_type
            ));
        }
    };

    match roundtrip {
        RoundtripResult2::Protobuf(buf) => conformance_response::Result::ProtobufPayload(buf),
        RoundtripResult2::Json(buf) => conformance_response::Result::JsonPayload(buf),
        RoundtripResult2::EncodeError(error) => {
            conformance_response::Result::SerializeError(error.to_string())
        }
        RoundtripResult2::DecodeError(error) => {
            conformance_response::Result::ParseError(error.to_string())
        }
        RoundtripResult2::Error(error) => {
            conformance_response::Result::RuntimeError(error.to_string())
        }
    }
}
