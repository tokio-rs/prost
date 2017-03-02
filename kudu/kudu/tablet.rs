//! ============================================================================
//!  Tablet Metadata
//! ============================================================================

#[derive(Debug, Message)]
pub struct ColumnDataPB {
    #[proto(tag="2")]
    pub block: Option<kudu::BlockIdPB>,
    /// REMOVED: optional ColumnSchemaPB OBSOLETE_schema = 3;
    #[proto(tag="4")]
    pub column_id: i32,
}
#[derive(Debug, Message)]
pub struct DeltaDataPB {
    #[proto(tag="2")]
    pub block: Option<kudu::BlockIdPB>,
}
#[derive(Debug, Message)]
pub struct RowSetDataPB {
    #[proto(tag="1")]
    pub id: u64,
    #[proto(tag="2")]
    pub last_durable_dms_id: i64,
    #[proto(tag="3")]
    pub columns: Vec<kudu::tablet::ColumnDataPB>,
    #[proto(tag="4")]
    pub redo_deltas: Vec<kudu::tablet::DeltaDataPB>,
    #[proto(tag="5")]
    pub undo_deltas: Vec<kudu::tablet::DeltaDataPB>,
    #[proto(tag="6")]
    pub bloom_block: Option<kudu::BlockIdPB>,
    #[proto(tag="7")]
    pub adhoc_index_block: Option<kudu::BlockIdPB>,
}
/// The super-block keeps track of the tablet data blocks.
/// A tablet contains one or more RowSets, which contain
/// a set of blocks (one for each column), a set of delta blocks
/// and optionally a block containing the bloom filter
/// and a block containing the compound-keys.
#[derive(Debug, Message)]
pub struct TabletSuperBlockPB {
    /// Table ID of the table this tablet is part of.
    #[proto(tag="1")]
    pub table_id: Vec<u8>,
    /// Tablet Id
    #[proto(tag="2")]
    pub tablet_id: Vec<u8>,
    /// The latest durable MemRowSet id
    #[proto(tag="3")]
    pub last_durable_mrs_id: i64,
    /// DEPRECATED.
    #[proto(tag="4")]
    pub start_key: Vec<u8>,
    /// DEPRECATED.
    #[proto(tag="5")]
    pub end_key: Vec<u8>,
    /// The partition of the table.
    #[proto(tag="13")]
    pub partition: Option<kudu::PartitionPB>,
    /// Tablet RowSets
    #[proto(tag="6")]
    pub rowsets: Vec<kudu::tablet::RowSetDataPB>,
    /// The latest schema
    /// TODO: maybe this should be TableSchemaPB? Need to actually put those attributes
    /// into use throughout the code. Using the simpler one for now.
    #[proto(tag="7")]
    pub table_name: String,
    #[proto(tag="8")]
    pub schema: Option<kudu::SchemaPB>,
    #[proto(tag="9")]
    pub schema_version: u32,
    /// The partition schema of the table.
    #[proto(tag="14")]
    pub partition_schema: Option<kudu::PartitionSchemaPB>,
    /// The current state of the tablet's data.
    #[proto(tag="10")]
    pub tablet_data_state: kudu::tablet::TabletDataState,
    /// Blocks that became orphans after flushing this superblock. In other
    /// words, the set difference of the blocks belonging to the previous
    /// superblock and this one.
    ///
    /// It's always safe to delete the blocks found here.
    #[proto(tag="11")]
    pub orphaned_blocks: Vec<kudu::BlockIdPB>,
    /// For tablets that have been tombstoned, stores the last OpId stored in the
    /// WAL before tombstoning.
    /// Only relevant for TOMBSTONED tablets.
    #[proto(tag="12")]
    pub tombstone_last_logged_opid: Option<kudu::consensus::OpId>,
}
/// State flags indicating whether the tablet is in the middle of being copied
/// and is therefore not possible to bring up, whether it has been deleted, or
/// whether the data is in a usable state.
#[derive(Clone, Copy, Debug, Enumeration)]
pub enum TabletDataState {
    TabletDataUnknown = 999,
    /// The tablet is set to TABLET_DATA_COPYING state when in the middle of
    /// copying data files from a remote peer. If a tablet server crashes with
    /// a tablet in this state, the tablet must be deleted and
    /// the Tablet Copy process must be restarted for that tablet.
    TabletDataCopying = 0,
    /// Fresh empty tablets and successfully copied tablets are set to the
    /// TABLET_DATA_READY state.
    TabletDataReady = 1,
    /// This tablet is in the process of being deleted.
    /// The tablet server should "roll forward" the deletion during boot,
    /// rather than trying to load the tablet.
    TabletDataDeleted = 2,
    /// The tablet has been deleted, and now just consists of a "tombstone".
    TabletDataTombstoned = 3,
}
/// The enum of tablet states.
/// Tablet states are sent in TabletReports and kept in TabletPeer.
#[derive(Clone, Copy, Debug, Enumeration)]
pub enum TabletStatePB {
    Unknown = 999,
    /// Tablet has not yet started.
    NotStarted = 5,
    /// Indicates the Tablet is bootstrapping, i.e. that the Tablet is not
    /// available for RPC.
    Bootstrapping = 0,
    /// Once the configuration phase is over Peers are in RUNNING state. In this
    /// state Peers are available for client RPCs.
    Running = 1,
    /// The tablet failed to for some reason. TabletPeer::error() will return
    /// the reason for the failure.
    Failed = 2,
    /// The Tablet is shutting down, and will not accept further requests.
    Quiescing = 3,
    /// The Tablet has been stopped.
    Shutdown = 4,
}
/// Stores the id of the MemRowSet (for inserts or mutations against MRS)
/// or of the (row set, delta ID) pair for mutations against a DiskRowSet.
/// -1 defaults here are so that, if a caller forgets to check has_mrs_id(),
/// they won't accidentally see real-looking (i.e 0) IDs.
#[derive(Debug, Message)]
pub struct MemStoreTargetPB {
    /// Either this field...
    #[proto(tag="1")]
    pub mrs_id: i64,
    /// ... or both of the following fields are set.
    #[proto(tag="2")]
    pub rs_id: i64,
    #[proto(tag="3")]
    pub dms_id: i64,
}
/// Stores the result of an Insert or Mutate.
#[derive(Debug, Message)]
pub struct OperationResultPB {
    /// set on replay if this operation was already flushed.
    #[proto(tag="1")]
    pub flushed: bool,
    /// set if this particular operation failed
    #[proto(tag="2")]
    pub failed_status: Option<kudu::AppStatusPB>,
    /// The stores that the operation affected.
    /// For INSERTs, this will always be just one store.
    /// For MUTATE, it may be more than one if the mutation arrived during
    /// a compaction.
    #[proto(tag="3")]
    pub mutated_stores: Vec<kudu::tablet::MemStoreTargetPB>,
}
/// The final result of a transaction, including the result of each individual
/// operation.
#[derive(Debug, Message)]
pub struct TxResultPB {
    /// all the operations in this transaction
    #[proto(tag="1")]
    pub ops: Vec<kudu::tablet::OperationResultPB>,
}
/// Delta statistics for a flushed deltastore
#[derive(Debug, Message)]
pub struct DeltaStatsPB {
    /// Number of deletes (deletes result in deletion of an entire row)
    #[proto(tag="1")]
    pub delete_count: i64,
    /// Number of reinserts.
    /// Optional for data format compatibility.
    #[proto(tag="6")]
    pub reinsert_count: i64,
    //! REMOVED: replaced by column_stats, which maps by column ID,
    //! whereas this older version mapped by index.
    //! repeated int64 per_column_update_count = 2;

    /// The min Timestamp that was stored in this delta.
    #[proto(tag="3", fixed)]
    pub min_timestamp: u64,
    /// The max Timestamp that was stored in this delta.
    #[proto(tag="4", fixed)]
    pub max_timestamp: u64,
    #[proto(tag="5")]
    pub column_stats: Vec<kudu::tablet::delta_stats_pb::ColumnStats>,
}
mod delta_stats_pb {
    /// Per-column statistics about this delta file.
    #[derive(Debug, Message)]
    pub struct ColumnStats {
        /// The column ID.
        #[proto(tag="1")]
        pub col_id: i32,
        /// The number of updates which refer to this column ID.
        #[proto(tag="2")]
        pub update_count: i64,
    }
}
#[derive(Debug, Message)]
pub struct TabletStatusPB {
    #[proto(tag="1")]
    pub tablet_id: String,
    #[proto(tag="2")]
    pub table_name: String,
    #[proto(tag="3")]
    pub state: kudu::tablet::TabletStatePB,
    #[proto(tag="8")]
    pub tablet_data_state: kudu::tablet::TabletDataState,
    #[proto(tag="4")]
    pub last_status: String,
    /// DEPRECATED.
    #[proto(tag="5")]
    pub start_key: Vec<u8>,
    /// DEPRECATED.
    #[proto(tag="6")]
    pub end_key: Vec<u8>,
    #[proto(tag="9")]
    pub partition: Option<kudu::PartitionPB>,
    #[proto(tag="7")]
    pub estimated_on_disk_size: i64,
}
