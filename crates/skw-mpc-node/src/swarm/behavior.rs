use libp2p::{
    swarm::{NetworkBehaviour},
    request_response,
};

// re-export
pub use self::skw_mpc_p2p_behavior::{SkwMpcP2pCodec, SkwMpcP2pProtocol, MpcP2pRequest, MpcP2pResponse};

#[derive(NetworkBehaviour)]
pub struct MpcSwarmBahavior {
    // node p2p behavior
    pub request_response: request_response::Behaviour<SkwMpcP2pCodec>,
}

// Sub protocol - p2p request-response
pub mod skw_mpc_p2p_behavior {
    use serde::{Serialize, Deserialize};

    use async_std::io;
    use async_trait::async_trait;
    use futures::prelude::*;
    use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed, ProtocolName};
    use libp2p::request_response::Codec;
    use skw_mpc_payload::{AuthHeader, PayloadHeader, CryptoHash};

    use crate::error::MpcNodeError;

    #[derive(Debug, Clone)]
    pub struct SkwMpcP2pProtocol();
    #[derive(Clone)]
    pub struct SkwMpcP2pCodec();

    // Serialized Form of raw request
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum MpcP2pRequest {
        StartJob {
            auth_header: AuthHeader,
            job_header: PayloadHeader,
        },
        RawMessage {
            payload: Vec<u8>, // Serialized Payload
        },
        RequestPartialSignature {
            payload_header_id: CryptoHash,
        }
    }

    // Serialized Form of raw response
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum MpcP2pResponse {
        StartJob {
            status: Result<(), MpcNodeError>
        },
        RawMessage {
            status: Result<(), MpcNodeError>,
            // NOTE: do we have any response to this? 
        },
        RequestPartialSignature {
            status: Result<Vec<u8>, MpcNodeError>,
        }
    }

    impl ProtocolName for SkwMpcP2pProtocol {
        fn protocol_name(&self) -> &[u8] {
            b"/skw-mpc-p2p/1"
        }
    }

    #[async_trait]
    impl Codec for SkwMpcP2pCodec {
        type Protocol = SkwMpcP2pProtocol;
        type Request = MpcP2pRequest;
        type Response = MpcP2pResponse;

        async fn read_request<T>(
            &mut self,
            _: &SkwMpcP2pProtocol,
            io: &mut T,
        ) -> io::Result<Self::Request>
        where
            T: AsyncRead + Unpin + Send,
        {
            let vec = read_length_prefixed(io, 1_000_000).await?;

            if vec.is_empty() {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            bincode::deserialize( &vec )
                .map_err(|_| io::ErrorKind::InvalidData.into() )
        }

        async fn read_response<T>(
            &mut self,
            _: &SkwMpcP2pProtocol,
            io: &mut T,
        ) -> io::Result<Self::Response>
        where
            T: AsyncRead + Unpin + Send,
        {
            let vec = read_length_prefixed(io, 10_240).await?; // update transfer maximum

            if vec.is_empty() {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }

            bincode::deserialize( &vec )
                .map_err(|_| io::ErrorKind::InvalidData.into() )
        }

        async fn write_request<T>(
            &mut self,
            _: &SkwMpcP2pProtocol,
            io: &mut T,
            raw: MpcP2pRequest,
        ) -> io::Result<()>
        where
            T: AsyncWrite + Unpin + Send,
        {
            let data = bincode::serialize(&raw).expect("request message to be valid");

            write_length_prefixed(io, data).await?;
            io.close().await?;

            Ok(())
        }

        async fn write_response<T>(
            &mut self,
            _: &SkwMpcP2pProtocol,
            io: &mut T,
            raw: MpcP2pResponse,
        ) -> io::Result<()>
        where
            T: AsyncWrite + Unpin + Send,
        {
            let data = bincode::serialize(&raw).expect("response message to be valid");
            write_length_prefixed(io, data).await?;
            io.close().await?;

            Ok(())
        }
    }
    
}