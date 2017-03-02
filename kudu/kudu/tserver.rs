/// Tablet-server specific errors use this protobuf.
#[derive(Debug, Message)]
pub struct TabletServerErrorPB {
    /// The error code.
    #[proto(tag="1")]
    pub code: kudu::tserver::tablet_server_error_pb::Code,
    /// The Status object for the error. This will include a textual
    /// message that may be more useful to present in log messages, etc,
    /// though its error code is less specific.
    #[proto(tag="2")]
    pub status: Option<kudu::AppStatusPB>,
}
mod tablet_server_error_pb {
    #[derive(Clone, Copy, Debug, Enumeration)]
    pub enum Code {
        /// An error which has no more specific error code.
        /// The code and message in 'status' may reveal more details.
        ///
        /// RPCs should avoid returning this, since callers will not be
        /// able to easily parse the error.
        UnknownError = 1,
        /// The schema provided for a request was not well-formed.
        InvalidSchema = 2,
        /// The row data provided for a request was not well-formed.
        InvalidRowBlock = 3,
        /// The mutations or mutation keys provided for a request were
        /// not well formed.
        InvalidMutation = 4,
        /// The schema provided for a request didn't match the actual
        /// schema of the tablet.
        MismatchedSchema = 5,
        /// The requested tablet_id is not currently hosted on this server.
        TabletNotFound = 6,
        /// A request was made against a scanner ID that was either never
        /// created or has expired.
        ScannerExpired = 7,
        /// An invalid scan was specified -- e.g the values passed for
        /// predicates were incorrect sizes.
        InvalidScanSpec = 8,
        /// The provided configuration was not well-formed and/or
        /// had a sequence number that was below the current config.
        InvalidConfig = 9,
        /// On a create tablet request, signals that the tablet already exists.
        TabletAlreadyExists = 10,
        /// If the tablet has a newer schema than the requested one the "alter"
        /// request will be rejected with this error.
        TabletHasANewerSchema = 11,
        /// The tablet is hosted on this server, but not in RUNNING state.
        TabletNotRunning = 12,
        /// Client requested a snapshot read but the snapshot was invalid.
        InvalidSnapshot = 13,
        /// An invalid scan call sequence ID was specified.
        InvalidScanCallSeqId = 14,
        /// This tserver is not the leader of the consensus configuration.
        NotTheLeader = 15,
        /// The destination UUID in the request does not match this server.
        WrongServerUuid = 16,
        /// The compare-and-swap specified by an atomic RPC operation failed.
        CasFailed = 17,
        /// The requested operation is already inprogress, e.g. TabletCopy.
        AlreadyInprogress = 18,
        /// The request is throttled.
        Throttled = 19,
    }
}
#[derive(Debug, Message)]
pub struct PingRequestPB {
}
#[derive(Debug, Message)]
pub struct PingResponsePB {
}
/// A batched set of insert/mutate requests.
#[derive(Debug, Message)]
pub struct WriteRequestPB {
    #[proto(tag="1")]
    pub tablet_id: Vec<u8>,
    /// The schema as seen by the client. This may be out-of-date, in which case
    /// it will be projected to the current schema automatically, with defaults/NULLs
    /// being filled in.
    #[proto(tag="2")]
    pub schema: Option<kudu::SchemaPB>,
    /// Operations to perform (insert/update/delete)
    #[proto(tag="3")]
    pub row_operations: Option<kudu::RowOperationsPB>,
    /// The required consistency mode for this write.
    #[proto(tag="4")]
    pub external_consistency_mode: kudu::ExternalConsistencyMode,
    /// A timestamp obtained by the client from a previous request.
    /// TODO crypto sign this and propagate the signature along with
    /// the timestamp.
    #[proto(tag="5", fixed)]
    pub propagated_timestamp: u64,
}
#[derive(Debug, Message)]
pub struct WriteResponsePB {
    /// If the entire WriteResponsePB request failed, the error status that
    /// caused the failure. This type of error is triggered for
    /// cases such as the tablet not being on this server, or the
    /// schema not matching. If any error specific to a given row
    /// occurs, this error will be recorded in per_row_errors below,
    /// even if all rows failed.
    #[proto(tag="1")]
    pub error: Option<kudu::tserver::TabletServerErrorPB>,
    #[proto(tag="2")]
    pub per_row_errors: Vec<kudu::tserver::write_response_pb::PerRowErrorPB>,
    /// The timestamp chosen by the server for this write.
    /// TODO KUDU-611 propagate timestamps with server signature.
    #[proto(tag="3", fixed)]
    pub timestamp: u64,
}
mod write_response_pb {
    /// If errors occurred with particular row operations, then the errors
    /// for those operations will be passed back in 'per_row_errors'.
    #[derive(Debug, Message)]
    pub struct PerRowErrorPB {
        /// The index of the row in the incoming batch.
        #[proto(tag="1")]
        pub row_index: i32,
        /// The error that occurred.
        #[proto(tag="2")]
        pub error: Option<kudu::AppStatusPB>,
    }
}
/// A list tablets request
#[derive(Debug, Message)]
pub struct ListTabletsRequestPB {
    /// Whether the server should include schema information in the response.
    /// These fields can be relatively large, so not including it can make this call
    /// less heavy-weight.
    #[proto(tag="1")]
    pub need_schema_info: bool,
}
/// A list tablets response
#[derive(Debug, Message)]
pub struct ListTabletsResponsePB {
    #[proto(tag="1")]
    pub error: Option<kudu::tserver::TabletServerErrorPB>,
    #[proto(tag="2")]
    pub status_and_schema: Vec<kudu::tserver::list_tablets_response_pb::StatusAndSchemaPB>,
}
mod list_tablets_response_pb {
    #[derive(Debug, Message)]
    pub struct StatusAndSchemaPB {
        #[proto(tag="1")]
        pub tablet_status: Option<kudu::tablet::TabletStatusPB>,
        /// 'schema' and 'partition_schema' will only be included if the original request
        /// set 'need_schema_info'.
        #[proto(tag="2")]
        pub schema: Option<kudu::SchemaPB>,
        #[proto(tag="3")]
        pub partition_schema: Option<kudu::PartitionSchemaPB>,
    }
}
/// DEPRECATED: Use ColumnPredicatePB
///
/// A range predicate on one of the columns in the underlying
/// data.
#[derive(Debug, Message)]
pub struct ColumnRangePredicatePB {
    #[proto(tag="1")]
    pub column: Option<kudu::ColumnSchemaPB>,
    /// These bounds should be encoded as follows:
    /// - STRING values: simply the exact string value for the bound.
    /// - other type: the canonical x86 in-memory representation -- eg for
    ///   uint32s, a little-endian value.
    ///
    /// Note that this predicate type should not be used for NULL data --
    /// NULL is defined to neither be greater than or less than other values
    /// for the comparison operator. We will eventually add a special
    /// predicate type for null-ness.
    ///
    /// Both bounds are inclusive.
    #[proto(tag="2")]
    pub lower_bound: Vec<u8>,
    #[proto(tag="3")]
    pub inclusive_upper_bound: Vec<u8>,
}
/// List of predicates used by the Java client. Will rapidly evolve into something more reusable
/// as a way to pass scanner configurations.
#[derive(Debug, Message)]
pub struct ColumnRangePredicateListPB {
    #[proto(tag="1")]
    pub range_predicates: Vec<kudu::tserver::ColumnRangePredicatePB>,
}
#[derive(Debug, Message)]
pub struct NewScanRequestPB {
    /// The tablet to scan.
    #[proto(tag="1")]
    pub tablet_id: Vec<u8>,
    /// The maximum number of rows to scan.
    /// The scanner will automatically stop yielding results and close
    /// itself after reaching this number of result rows.
    #[proto(tag="2")]
    pub limit: u64,
    /// DEPRECATED: use column_predicates field.
    ///
    /// Any column range predicates to enforce.
    #[proto(tag="3")]
    pub DEPRECATED_range_predicates: Vec<kudu::tserver::ColumnRangePredicatePB>,
    /// Column predicates to enforce.
    #[proto(tag="13")]
    pub column_predicates: Vec<kudu::ColumnPredicatePB>,
    /// Encoded primary key to begin scanning at (inclusive).
    #[proto(tag="8")]
    pub start_primary_key: Vec<u8>,
    /// Encoded primary key to stop scanning at (exclusive).
    #[proto(tag="9")]
    pub stop_primary_key: Vec<u8>,
    /// Which columns to select.
    /// if this is an empty list, no data will be returned, but the num_rows
    /// field of the returned RowBlock will indicate how many rows passed
    /// the predicates. Note that in some cases, the scan may still require
    /// multiple round-trips, and the caller must aggregate the counts.
    #[proto(tag="4")]
    pub projected_columns: Vec<kudu::ColumnSchemaPB>,
    /// The read mode for this scan request.
    /// See common.proto for further information about read modes.
    #[proto(tag="5")]
    pub read_mode: kudu::ReadMode,
    /// The requested snapshot timestamp. This is only used
    /// when the read mode is set to READ_AT_SNAPSHOT.
    #[proto(tag="6", fixed)]
    pub snap_timestamp: u64,
    /// Sent by clients which previously executed CLIENT_PROPAGATED writes.
    /// This updates the server's time so that no transaction will be assigned
    /// a timestamp lower than or equal to 'previous_known_timestamp'
    #[proto(tag="7", fixed)]
    pub propagated_timestamp: u64,
    /// Whether data blocks will be cached when read from the files or discarded after use.
    /// Disable this to lower cache churn when doing large scans.
    #[proto(tag="10")]
    pub cache_blocks: bool,
    /// Whether to order the returned rows by primary key.
    /// This is used for scanner fault-tolerance.
    #[proto(tag="11")]
    pub order_mode: kudu::OrderMode,
    /// If retrying a scan, the final primary key retrieved in the previous scan
    /// attempt. If set, this will take precedence over the `start_primary_key`
    /// field, and functions as an exclusive start primary key.
    #[proto(tag="12")]
    pub last_primary_key: Vec<u8>,
}
/// A scan request. Initially, it should specify a scan. Later on, you
/// can use the scanner id returned to fetch result batches with a different
/// scan request.
///
/// The scanner will remain open if there are more results, and it's not
/// asked to be closed explicitly. Some errors on the Tablet Server may
/// close the scanner automatically if the scanner state becomes
/// inconsistent.
///
/// Clients may choose to retry scan requests that fail to complete (due to, for
/// example, a timeout or network error). If a scan request completes with an
/// error result, the scanner should be closed by the client.
///
/// You can fetch the results and ask the scanner to be closed to save
/// a trip if you are not interested in remaining results.
///
/// This is modeled somewhat after HBase's scanner API.
#[derive(Debug, Message)]
pub struct ScanRequestPB {
    /// If continuing an existing scan, then you must set scanner_id.
    /// Otherwise, you must set 'new_scan_request'.
    #[proto(tag="1")]
    pub scanner_id: Vec<u8>,
    #[proto(tag="2")]
    pub new_scan_request: Option<kudu::tserver::NewScanRequestPB>,
    /// The sequence ID of this call. The sequence ID should start at 0
    /// with the request for a new scanner, and after each successful request,
    /// the client should increment it by 1. When retrying a request, the client
    /// should _not_ increment this value. If the server detects that the client
    /// missed a chunk of rows from the middle of a scan, it will respond with an
    /// error.
    #[proto(tag="3")]
    pub call_seq_id: u32,
    /// The maximum number of bytes to send in the response.
    /// This is a hint, not a requirement: the server may send
    /// arbitrarily fewer or more bytes than requested.
    #[proto(tag="4")]
    pub batch_size_bytes: u32,
    /// If set, the server will close the scanner after responding to
    /// this request, regardless of whether all rows have been delivered.
    /// In order to simply close a scanner without selecting any rows, you
    /// may set batch_size_bytes to 0 in conjunction with setting this flag.
    #[proto(tag="5")]
    pub close_scanner: bool,
}
/// RPC's resource metrics.
#[derive(Debug, Message)]
pub struct ResourceMetricsPB {
    /// all metrics MUST be the type of int64.
    #[proto(tag="1")]
    pub cfile_cache_miss_bytes: i64,
    #[proto(tag="2")]
    pub cfile_cache_hit_bytes: i64,
}
#[derive(Debug, Message)]
pub struct ScanResponsePB {
    /// The error, if an error occurred with this request.
    #[proto(tag="1")]
    pub error: Option<kudu::tserver::TabletServerErrorPB>,
    /// When a scanner is created, returns the scanner ID which may be used
    /// to pull new rows from the scanner.
    #[proto(tag="2")]
    pub scanner_id: Vec<u8>,
    /// Set to true to indicate that there may be further results to be fetched
    /// from this scanner. If the scanner has no more results, then the scanner
    /// ID will become invalid and cannot continue to be used.
    ///
    /// Note that if a scan returns no results, then the initial response from
    /// the first RPC may return false in this flag, in which case there will
    /// be no scanner ID assigned.
    #[proto(tag="3")]
    pub has_more_results: bool,
    /// The block of returned rows.
    ///
    /// NOTE: the schema-related fields will not be present in this row block.
    /// The schema will match the schema requested by the client when it created
    /// the scanner.
    #[proto(tag="4")]
    pub data: Option<kudu::RowwiseRowBlockPB>,
    /// The snapshot timestamp at which the scan was executed. This is only set
    /// in the first response (i.e. the response to the request that had
    /// 'new_scan_request' set) and only for READ_AT_SNAPSHOT scans.
    #[proto(tag="6", fixed)]
    pub snap_timestamp: u64,
    /// If this is a fault-tolerant scanner, this is set to the encoded primary
    /// key of the last row returned in the response.
    #[proto(tag="7")]
    pub last_primary_key: Vec<u8>,
    /// The resource usage of this RPC.
    #[proto(tag="8")]
    pub resource_metrics: Option<kudu::tserver::ResourceMetricsPB>,
    /// The server's time upon sending out the scan response. Should always
    /// be greater than the scan timestamp.
    #[proto(tag="9", fixed)]
    pub propagated_timestamp: u64,
}
/// A scanner keep-alive request.
/// Updates the scanner access time, increasing its time-to-live.
#[derive(Debug, Message)]
pub struct ScannerKeepAliveRequestPB {
    #[proto(tag="1")]
    pub scanner_id: Vec<u8>,
}
#[derive(Debug, Message)]
pub struct ScannerKeepAliveResponsePB {
    /// The error, if an error occurred with this request.
    #[proto(tag="1")]
    pub error: Option<kudu::tserver::TabletServerErrorPB>,
}
#[derive(Clone, Copy, Debug, Enumeration)]
pub enum TabletServerFeatures {
    UnknownFeature = 0,
    ColumnPredicates = 1,
}
