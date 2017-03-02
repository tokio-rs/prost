#[derive(Clone, Copy, Debug, Enumeration)]
pub enum CompressionType {
    UnknownCompression = 999,
    DefaultCompression = 0,
    NoCompression = 1,
    Snappy = 2,
    Lz4 = 3,
    Zlib = 4,
}

//! ============================================================================
//!  Protobuf container metadata
//! ============================================================================

/// Supplemental protobuf container header, after the main header (see
/// pb_util.h for details).
#[derive(Debug, Message)]
pub struct ContainerSupHeaderPB {
    /// The protobuf schema for the messages expected in this container.
    ///
    /// This schema is complete, that is, it includes all of its dependencies
    /// (i.e. other schemas defined in .proto files imported by this schema's
    /// .proto file).
    #[proto(tag="1")]
    pub protos: Option<google::protobuf::FileDescriptorSet>,
    /// The PB message type expected in each data entry in this container. Must
    /// be fully qualified (i.e. kudu.tablet.TabletSuperBlockPB).
    #[proto(tag="2")]
    pub pb_type: String,
}
/// TODO: Differentiate between the schema attributes
/// that are only relevant to the server (e.g.,
/// encoding and compression) and those that also
/// matter to the client.
#[derive(Debug, Message)]
pub struct ColumnSchemaPB {
    #[proto(tag="1")]
    pub id: u32,
    #[proto(tag="2")]
    pub name: String,
    #[proto(tag="3")]
    pub type: kudu::DataType,
    #[proto(tag="4")]
    pub is_key: bool,
    #[proto(tag="5")]
    pub is_nullable: bool,
    /// Default values.
    /// NOTE: as far as clients are concerned, there is only one
    /// "default value" of a column. The read/write defaults are used
    /// internally and should not be exposed by any public client APIs.
    ///
    /// When passing schemas to the master for create/alter table,
    /// specify the default in 'read_default_value'.
    ///
    /// Contrary to this, when the client opens a table, it will receive
    /// both the read and write defaults, but the *write* default is
    /// what should be exposed as the "current" default.
    #[proto(tag="6")]
    pub read_default_value: Vec<u8>,
    #[proto(tag="7")]
    pub write_default_value: Vec<u8>,
    /// The following attributes refer to the on-disk storage of the column.
    /// They won't always be set, depending on context.
    #[proto(tag="8")]
    pub encoding: kudu::EncodingType,
    #[proto(tag="9")]
    pub compression: kudu::CompressionType,
    #[proto(tag="10")]
    pub cfile_block_size: i32,
}
#[derive(Debug, Message)]
pub struct SchemaPB {
    #[proto(tag="1")]
    pub columns: Vec<kudu::ColumnSchemaPB>,
}
#[derive(Debug, Message)]
pub struct HostPortPB {
    #[proto(tag="1")]
    pub host: String,
    #[proto(tag="2")]
    pub port: u32,
}
/// The serialized format of a Kudu table partition schema.
#[derive(Debug, Message)]
pub struct PartitionSchemaPB {
    #[proto(tag="1")]
    pub hash_bucket_schemas: Vec<kudu::partition_schema_pb::HashBucketSchemaPB>,
    #[proto(tag="2")]
    pub range_schema: Option<kudu::partition_schema_pb::RangeSchemaPB>,
}
mod partition_schema_pb {
    /// A column identifier for partition schemas. In general, the name will be
    /// used when a client creates the table since column IDs are assigned by the
    /// master. All other uses of partition schemas will use the numeric column ID.
    #[derive(Debug, Message)]
    pub struct ColumnIdentifierPB {
        #[proto(tag="1")]
        pub id: i32,
        #[proto(tag="2")]
        pub name: String,
    }
    #[derive(Debug, Message)]
    pub struct RangeSchemaPB {
        /// Column identifiers of columns included in the range. All columns must be
        /// a component of the primary key.
        #[proto(tag="1")]
        pub columns: Vec<kudu::partition_schema_pb::ColumnIdentifierPB>,
    }
    #[derive(Debug, Message)]
    pub struct HashBucketSchemaPB {
        /// Column identifiers of columns included in the hash. Every column must be
        /// a component of the primary key.
        #[proto(tag="1")]
        pub columns: Vec<kudu::partition_schema_pb::ColumnIdentifierPB>,
        /// Number of buckets into which columns will be hashed. Must be at least 2.
        #[proto(tag="2")]
        pub num_buckets: i32,
        /// Seed value for hash calculation. Administrators may set a seed value
        /// on a per-table basis in order to randomize the mapping of rows to
        /// buckets. Setting a seed provides some amount of protection against denial
        /// of service attacks when the hash bucket columns contain user provided
        /// input.
        #[proto(tag="3")]
        pub seed: u32,
        /// The hash algorithm to use for calculating the hash bucket.
        #[proto(tag="4")]
        pub hash_algorithm: kudu::partition_schema_pb::hash_bucket_schema_pb::HashAlgorithm,
    }
    mod hash_bucket_schema_pb {
        #[derive(Clone, Copy, Debug, Enumeration)]
        pub enum HashAlgorithm {
            Unknown = 0,
            MurmurHash2 = 1,
        }
    }
}
/// The serialized format of a Kudu table partition.
#[derive(Debug, Message)]
pub struct PartitionPB {
    /// The hash buckets of the partition. The number of hash buckets must match
    /// the number of hash bucket components in the partition's schema.
    #[proto(tag="1")]
    pub hash_buckets: Vec<i32>,
    /// The encoded start partition key (inclusive).
    #[proto(tag="2")]
    pub partition_key_start: Vec<u8>,
    /// The encoded end partition key (exclusive).
    #[proto(tag="3")]
    pub partition_key_end: Vec<u8>,
}
/// A predicate that can be applied on a Kudu column.
#[derive(Debug, Message)]
pub struct ColumnPredicatePB {
    /// The predicate column name.
    #[proto(tag="1")]
    pub column: String,
    #[proto(tag="2")]
    pub range: Option<kudu::column_predicate_pb::Range>,
    #[proto(tag="3")]
    pub equality: Option<kudu::column_predicate_pb::Equality>,
    #[proto(tag="4")]
    pub is_not_null: Option<kudu::column_predicate_pb::IsNotNull>,
    #[proto(tag="5")]
    pub in_list: Option<kudu::column_predicate_pb::InList>,
    #[proto(tag="6")]
    pub is_null: Option<kudu::column_predicate_pb::IsNull>,
}
mod column_predicate_pb {
    #[derive(Debug, Message)]
    pub struct Range {
        //! Bounds should be encoded as follows:
        //! - STRING/BINARY values: simply the exact string value for the bound.
        //! - other type: the canonical x86 in-memory representation -- eg for
        //!   uint32s, a little-endian value.
        //!
        //! Note that this predicate type should not be used for NULL data --
        //! NULL is defined to neither be greater than or less than other values
        //! for the comparison operator. We will eventually add a special
        //! predicate type for null-ness.

        /// The inclusive lower bound.
        #[proto(tag="1")]
        pub lower: Vec<u8>,
        /// The exclusive upper bound.
        #[proto(tag="2")]
        pub upper: Vec<u8>,
    }
    #[derive(Debug, Message)]
    pub struct Equality {
        /// The inclusive lower bound. See comment in Range for notes on the
        /// encoding.
        #[proto(tag="1")]
        pub value: Vec<u8>,
    }
    #[derive(Debug, Message)]
    pub struct InList {
        /// A list of values for the field. See comment in Range for notes on
        /// the encoding.
        #[proto(tag="1")]
        pub values: Vec<Vec<u8>>,
    }
    #[derive(Debug, Message)]
    pub struct IsNotNull {
    }
    #[derive(Debug, Message)]
    pub struct IsNull {
    }
}
/// If you add a new type keep in mind to add it to the end
/// or update AddMapping() functions like the one in key_encoder.cc
/// that have a vector that maps the protobuf tag with the index.
#[derive(Clone, Copy, Debug, Enumeration)]
pub enum DataType {
    UnknownData = 999,
    Uint8 = 0,
    Int8 = 1,
    Uint16 = 2,
    Int16 = 3,
    Uint32 = 4,
    Int32 = 5,
    Uint64 = 6,
    Int64 = 7,
    String = 8,
    Bool = 9,
    Float = 10,
    Double = 11,
    Binary = 12,
    UnixtimeMicros = 13,
}
#[derive(Clone, Copy, Debug, Enumeration)]
pub enum EncodingType {
    UnknownEncoding = 999,
    AutoEncoding = 0,
    PlainEncoding = 1,
    PrefixEncoding = 2,
    /// GROUP_VARINT encoding is deprecated and no longer implemented.
    GroupVarint = 3,
    Rle = 4,
    DictEncoding = 5,
    BitShuffle = 6,
}
/// The external consistency mode for client requests.
/// This defines how transactions and/or sequences of operations that touch
/// several TabletServers, in different machines, can be observed by external
/// clients.
///
/// Note that ExternalConsistencyMode makes no guarantee on atomicity, i.e.
/// no sequence of operations is made atomic (or transactional) just because
/// an external consistency mode is set.
/// Note also that ExternalConsistencyMode has no implication on the
/// consistency between replicas of the same tablet.
#[derive(Clone, Copy, Debug, Enumeration)]
pub enum ExternalConsistencyMode {
    UnknownExternalConsistencyMode = 0,
    /// The response to any write will contain a timestamp.
    /// Any further calls from the same client to other servers will update
    /// those servers with that timestamp. The user will make sure that the
    /// timestamp is propagated through back-channels to other
    /// KuduClient's.
    ///
    /// WARNING: Failure to propagate timestamp information through
    /// back-channels will negate any external consistency guarantee under this
    /// mode.
    ///
    /// Example:
    /// 1 - Client A executes operation X in Tablet A
    /// 2 - Afterwards, Client A executes operation Y in Tablet B
    ///
    ///
    /// Client B may observe the following operation sequences:
    /// {}, {X}, {X Y}
    ///
    /// This is the default mode.
    ClientPropagated = 1,
    /// The server will guarantee that each transaction is externally
    /// consistent by making sure that none of its results are visible
    /// until every Kudu server agrees that the transaction is in the past.
    /// The client is not obligated to forward timestamp information
    /// through back-channels.
    ///
    /// WARNING: Depending on the clock synchronization state of TabletServers
    /// this may imply considerable latency. Moreover operations with
    /// COMMIT_WAIT requested external consistency will outright fail if
    /// TabletServer clocks are either unsynchronized or synchronized but
    /// with a maximum error which surpasses a pre-configured one.
    ///
    /// Example:
    /// - Client A executes operation X in Tablet A
    /// - Afterwards, Client A executes operation Y in Tablet B
    ///
    ///
    /// Client B may observe the following operation sequences:
    /// {}, {X}, {X Y}
    CommitWait = 2,
}
/// The possible read modes for clients.
/// Clients set these in Scan requests.
/// The server keeps 2 snapshot boundaries:
/// - The earliest snapshot: this corresponds to the earliest kept undo records
///   in the tablet, meaning the current state (Base) can be undone up to
///   this snapshot.
/// - The latest snapshot: This corresponds to the instant beyond which no
///   no transaction will have an earlier timestamp. Usually this corresponds
///   to whatever clock->Now() returns, but can be higher if the client propagates
///   a timestamp (see below).
#[derive(Clone, Copy, Debug, Enumeration)]
pub enum ReadMode {
    UnknownReadMode = 0,
    /// When READ_LATEST is specified the server will execute the read independently
    /// of the clock and will always return all visible writes at the time the request
    /// was received. This type of read does not return a snapshot timestamp since
    /// it might not be repeatable, i.e. a later read executed at the same snapshot
    /// timestamp might yield rows that were committed by in-flight transactions.
    ///
    /// This is the default mode.
    ReadLatest = 1,
    /// When READ_AT_SNAPSHOT is specified the server will attempt to perform a read
    /// at the required snapshot. If no snapshot is defined the server will take the
    /// current time as the snapshot timestamp. Snapshot reads are repeatable, i.e.
    /// all future reads at the same timestamp will yield the same rows. This is
    /// performed at the expense of waiting for in-flight transactions whose timestamp
    /// is lower than the snapshot's timestamp to complete.
    ///
    /// When mixing reads and writes clients that specify COMMIT_WAIT as their
    /// external consistency mode and then use the returned write_timestamp to
    /// to perform snapshot reads are guaranteed that that snapshot time is
    /// considered in the past by all servers and no additional action is
    /// necessary. Clients using CLIENT_PROPAGATED however must forcibly propagate
    /// the timestamps even at read time, so that the server will not generate
    /// any more transactions before the snapshot requested by the client.
    /// The latter option is implemented by allowing the client to specify one or
    /// two timestamps, the first one obtained from the previous CLIENT_PROPAGATED
    /// write, directly or through back-channels, must be signed and will be
    /// checked by the server. The second one, if defined, is the actual snapshot
    /// read time. When selecting both the latter must be lower than or equal to
    /// the former.
    /// TODO implement actually signing the propagated timestamp.
    ReadAtSnapshot = 2,
}
/// The possible order modes for clients.
/// Clients specify these in new scan requests.
/// Ordered scans are fault-tolerant, and can be retried elsewhere in the case
/// of tablet server failure. However, ordered scans impose additional overhead
/// since the tablet server needs to sort the result rows.
#[derive(Clone, Copy, Debug, Enumeration)]
pub enum OrderMode {
    UnknownOrderMode = 0,
    /// This is the default order mode.
    Unordered = 1,
    Ordered = 2,
}
/// Error status returned by any RPC method.
/// Every RPC method which could generate an application-level error
/// should have this (or a more complex error result) as an optional field
/// in its response.
///
/// This maps to kudu::Status in C++ and org.apache.kudu.Status in Java.
#[derive(Debug, Message)]
pub struct AppStatusPB {
    #[proto(tag="1")]
    pub code: kudu::app_status_pb::ErrorCode,
    #[proto(tag="2")]
    pub message: String,
    #[proto(tag="4")]
    pub posix_code: i32,
}
mod app_status_pb {
    #[derive(Clone, Copy, Debug, Enumeration)]
    pub enum ErrorCode {
        UnknownError = 999,
        Ok = 0,
        NotFound = 1,
        Corruption = 2,
        NotSupported = 3,
        InvalidArgument = 4,
        IoError = 5,
        AlreadyPresent = 6,
        RuntimeError = 7,
        NetworkError = 8,
        IllegalState = 9,
        NotAuthorized = 10,
        Aborted = 11,
        RemoteError = 12,
        ServiceUnavailable = 13,
        TimedOut = 14,
        Uninitialized = 15,
        ConfigurationError = 16,
        Incomplete = 17,
        EndOfFile = 18,
    }
}
/// Uniquely identify a particular instance of a particular server in the
/// cluster.
#[derive(Debug, Message)]
pub struct NodeInstancePB {
    /// Unique ID which is created when the server is first started
    /// up. This is stored persistently on disk.
    #[proto(tag="1")]
    pub permanent_uuid: Vec<u8>,
    /// Sequence number incremented on every start-up of the server.
    /// This makes it easy to detect when an instance has restarted (and
    /// thus can be assumed to have forgotten any soft state it had in
    /// memory).
    ///
    /// On a freshly initialized server, the first sequence number
    /// should be 0.
    #[proto(tag="2")]
    pub instance_seqno: i64,
}
/// Some basic properties common to both masters and tservers.
/// They are guaranteed not to change unless the server is restarted.
#[derive(Debug, Message)]
pub struct ServerRegistrationPB {
    #[proto(tag="1")]
    pub rpc_addresses: Vec<kudu::HostPortPB>,
    #[proto(tag="2")]
    pub http_addresses: Vec<kudu::HostPortPB>,
    #[proto(tag="3")]
    pub software_version: String,
    /// True if HTTPS has been enabled for the web interface.
    /// In this case, https:// URLs should be generated for the above
    /// 'http_addresses' field.
    #[proto(tag="4")]
    pub https_enabled: bool,
}
#[derive(Debug, Message)]
pub struct ServerEntryPB {
    /// If there is an error communicating with the server (or retrieving
    /// the server registration on the server itself), this field will be
    /// set to contain the error.
    ///
    /// All subsequent fields are optional, as they may not be set if
    /// an error is encountered communicating with the individual server.
    #[proto(tag="1")]
    pub error: Option<kudu::AppStatusPB>,
    #[proto(tag="2")]
    pub instance_id: Option<kudu::NodeInstancePB>,
    #[proto(tag="3")]
    pub registration: Option<kudu::ServerRegistrationPB>,
    /// If an error has occured earlier in the RPC call, the role
    /// may be not be set.
    #[proto(tag="4")]
    pub role: kudu::consensus::raft_peer_pb::Role,
}
/// A row block in which each row is stored contiguously.
#[derive(Debug, Message)]
pub struct RowwiseRowBlockPB {
    /// The number of rows in the block. This can typically be calculated
    /// by dividing rows.size() by the width of the row, but in the case that
    /// the client is scanning an empty projection (i.e a COUNT(*)), this
    /// field is the only way to determine how many rows were returned.
    #[proto(tag="1")]
    pub num_rows: i32,
    /// Sidecar index for the row data.
    ///
    /// In the sidecar, each row is stored in the same in-memory format
    /// as kudu::ContiguousRow (i.e the raw unencoded data followed by
    /// a null bitmap).
    ///
    /// The data for NULL cells will be present with undefined contents --
    /// typically it will be filled with \x00s but this is not guaranteed,
    /// and clients may choose to initialize NULL cells with whatever they
    /// like. Setting to some constant improves RPC compression, though.
    ///
    /// Any pointers are made relative to the beginning of the indirect
    /// data sidecar.
    ///
    /// See rpc/rpc_sidecar.h for more information on where the data is
    /// actually stored.
    #[proto(tag="2")]
    pub rows_sidecar: i32,
    /// Sidecar index for the indirect data.
    ///
    /// In the sidecar, "indirect" data types in the block are stored
    /// contiguously. For example, STRING values in the block will be
    /// stored using the normal Slice in-memory format, except that
    /// instead of being pointers in RAM, the pointer portion will be an
    /// offset into this protobuf field.
    #[proto(tag="3")]
    pub indirect_data_sidecar: i32,
}
/// A set of operations (INSERT, UPDATE, UPSERT, or DELETE) to apply to a table,
/// or the set of split rows and range bounds when creating or altering table.
/// Range bounds determine the boundaries of range partitions during table
/// creation, split rows further subdivide the ranges into more partitions.
#[derive(Debug, Message)]
pub struct RowOperationsPB {
    /// The row data for each operation is stored in the following format:
    ///
    /// [operation type] (one byte):
    ///   A single-byte field which determines the type of operation. The values are
    ///   based on the 'Type' enum above.
    /// [column isset bitmap]   (one bit for each column in the Schema, rounded to nearest byte)
    ///   A set bit in this bitmap indicates that the user has specified the given column
    ///   in the row. This indicates that the column will be present in the data to follow.
    /// [null bitmap]           (one bit for each Schema column, rounded to nearest byte)
    ///   A set bit in this bitmap indicates that the given column is NULL.
    ///   This is only present if there are any nullable columns.
    /// [column data]
    ///   For each column which is set and not NULL, the column's data follows. The data
    ///   format of each cell is the canonical in-memory format (eg little endian).
    ///   For string data, the pointers are relative to 'indirect_data'.
    ///
    /// The rows are concatenated end-to-end with no padding/alignment.
    #[proto(tag="2")]
    pub rows: Vec<u8>,
    #[proto(tag="3")]
    pub indirect_data: Vec<u8>,
}
mod row_operations_pb {
    #[derive(Clone, Copy, Debug, Enumeration)]
    pub enum Type {
        Unknown = 0,
        Insert = 1,
        Update = 2,
        Delete = 3,
        Upsert = 5,
        /// Used when specifying split rows on table creation.
        SplitRow = 4,
        /// Used when specifying an inclusive lower bound range on table creation.
        /// Should be followed by the associated upper bound. If all values are
        /// missing, then signifies unbounded.
        RangeLowerBound = 6,
        /// Used when specifying an exclusive upper bound range on table creation.
        /// Should be preceded by the associated lower bound. If all values are
        /// missing, then signifies unbounded.
        RangeUpperBound = 7,
        /// Used when specifying an exclusive lower bound range on table creation.
        /// Should be followed by the associated upper bound. If all values are
        /// missing, then signifies unbounded.
        ExclusiveRangeLowerBound = 8,
        /// Used when specifying an inclusive upper bound range on table creation.
        /// Should be preceded by the associated lower bound. If all values are
        /// missing, then signifies unbounded.
        InclusiveRangeUpperBound = 9,
    }
}
//! ============================================================================
//!  Local file system metadata
//! ============================================================================

/// When any server initializes a new filesystem (eg a new node is created in the
/// cluster), it creates this structure and stores it persistently.
#[derive(Debug, Message)]
pub struct InstanceMetadataPB {
    /// The UUID which is assigned when the instance is first formatted. This uniquely
    /// identifies the node in the cluster.
    #[proto(tag="1")]
    pub uuid: Vec<u8>,
    /// Human-readable string indicating when and where the node was first
    /// initialized.
    #[proto(tag="2")]
    pub format_stamp: String,
}
/// Describes a collection of filesystem path instances and the membership of a
/// particular instance in the collection.
///
/// In a healthy filesystem (see below), a path instance can be referred to via
/// its UUID's position in all_uuids instead of via the UUID itself. This is
/// useful when there are many such references, as the position is much shorter
/// than the UUID.
#[derive(Debug, Message)]
pub struct PathSetPB {
    /// Globally unique identifier for this path instance.
    #[proto(tag="1")]
    pub uuid: Vec<u8>,
    /// All UUIDs in this path instance set. In a healthy filesystem:
    /// 1. There exists an on-disk PathInstanceMetadataPB for each listed UUID, and
    /// 2. Every PathSetPB contains an identical copy of all_uuids.
    #[proto(tag="2")]
    pub all_uuids: Vec<Vec<u8>>,
}
/// A filesystem instance can contain multiple paths. One of these structures
/// is persisted in each path when the filesystem instance is created.
#[derive(Debug, Message)]
pub struct PathInstanceMetadataPB {
    /// Describes this path instance as well as all of the other path instances
    /// that, taken together, describe a complete set.
    #[proto(tag="1")]
    pub path_set: Option<kudu::PathSetPB>,
    /// Textual representation of the block manager that formatted this path.
    #[proto(tag="2")]
    pub block_manager_type: String,
    /// Block size on the filesystem where this instance was created. If the
    /// instance (and its data) are ever copied to another location, the block
    /// size in that location must be the same.
    #[proto(tag="3")]
    pub filesystem_block_size_bytes: u64,
}
#[derive(Debug, Message)]
pub struct BlockIdPB {
    #[proto(tag="1", fixed)]
    pub id: u64,
}
/// An element found in a container metadata file of the log-backed block
/// storage implementation.
///
/// Each one tracks the existence (creation) or non-existence (deletion)
/// of a particular block. They are written sequentially, with subsequent
/// messages taking precedence over earlier ones (e.g. "CREATE foo" followed
/// by "DELETE foo" means that block foo does not exist).
#[derive(Debug, Message)]
pub struct BlockRecordPB {
    /// The unique identifier of the block.
    #[proto(tag="1")]
    pub block_id: Option<kudu::BlockIdPB>,
    /// Whether this is a CREATE or a DELETE.
    #[proto(tag="2")]
    pub op_type: kudu::BlockRecordType,
    /// The time at which this block record was created, expressed in terms of
    /// microseconds since the epoch.
    #[proto(tag="3")]
    pub timestamp_us: u64,
    /// The offset of the block in the container data file.
    ///
    /// Required for CREATE.
    #[proto(tag="4")]
    pub offset: i64,
    /// The length of the block in the container data file.
    ///
    /// Required for CREATE.
    #[proto(tag="5")]
    pub length: i64,
}
/// The kind of record.
#[derive(Clone, Copy, Debug, Enumeration)]
pub enum BlockRecordType {
    Unknown = 0,
    Create = 1,
    Delete = 2,
}
