pub mod convert;

pub mod proto {
    tonic::include_proto!("ennio_node");
}

pub use proto::ennio_node_client::EnnioNodeClient;
pub use proto::ennio_node_server::{EnnioNode, EnnioNodeServer};
pub use proto::{
    CreateRuntimeRequest, CreateRuntimeResponse, CreateWorkspaceRequest, CreateWorkspaceResponse,
    DestroyRuntimeRequest, DestroyRuntimeResponse, DestroyWorkspaceRequest,
    DestroyWorkspaceResponse, GetOutputRequest, GetOutputResponse, HeartbeatRequest,
    HeartbeatResponse, IsAliveRequest, IsAliveResponse, ProtoRuntimeHandle, SendMessageRequest,
    SendMessageResponse, ShutdownRequest, ShutdownResponse,
};
