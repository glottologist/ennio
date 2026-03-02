use std::collections::HashMap;

use ennio_core::runtime::{RuntimeCreateConfig, RuntimeHandle};

use crate::proto::{CreateRuntimeRequest, ProtoRuntimeHandle};

impl From<&RuntimeCreateConfig> for CreateRuntimeRequest {
    fn from(config: &RuntimeCreateConfig) -> Self {
        Self {
            session_id: config.session_id.to_string(),
            launch_command: config.launch_command.clone(), // clone: proto message needs owned String
            env: config.env.clone(), // clone: proto message needs owned HashMap
            cwd: config.cwd.clone(), // clone: proto message needs owned String
            session_name: config.session_name.clone(), // clone: proto message needs owned String
        }
    }
}

impl From<&RuntimeHandle> for ProtoRuntimeHandle {
    fn from(handle: &RuntimeHandle) -> Self {
        let data: HashMap<String, String> = handle
            .data
            .iter()
            .map(|(k, v)| {
                let s = match v {
                    serde_json::Value::String(s) => s.clone(), // clone: proto map needs owned String
                    other => other.to_string(),
                };
                (k.clone(), s) // clone: proto map needs owned keys
            })
            .collect();

        Self {
            id: handle.id.clone(), // clone: proto message needs owned String
            runtime_name: handle.runtime_name.clone(), // clone: proto message needs owned String
            data,
        }
    }
}

impl From<ProtoRuntimeHandle> for RuntimeHandle {
    fn from(proto: ProtoRuntimeHandle) -> Self {
        let data: HashMap<String, serde_json::Value> = proto
            .data
            .into_iter()
            .map(|(k, v)| (k, serde_json::Value::String(v)))
            .collect();

        Self {
            id: proto.id,
            runtime_name: proto.runtime_name,
            data,
        }
    }
}

impl From<&ProtoRuntimeHandle> for RuntimeHandle {
    fn from(proto: &ProtoRuntimeHandle) -> Self {
        let data: HashMap<String, serde_json::Value> = proto
            .data
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone()))) // clone: building owned HashMap from reference
            .collect();

        Self {
            id: proto.id.clone(), // clone: building owned struct from reference
            runtime_name: proto.runtime_name.clone(), // clone: building owned struct from reference
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use proptest::prelude::*;

    use ennio_core::id::SessionId;
    use ennio_core::runtime::{RuntimeCreateConfig, RuntimeHandle};

    use crate::proto::{CreateRuntimeRequest, ProtoRuntimeHandle};

    proptest! {
        #[test]
        fn runtime_handle_roundtrip(
            id in "[a-z0-9-]{1,32}",
            runtime_name in "[a-z0-9-]{1,32}",
        ) {
            let handle = RuntimeHandle {
                id: id.clone(),
                runtime_name: runtime_name.clone(),
                data: HashMap::new(),
            };

            let proto: ProtoRuntimeHandle = (&handle).into();
            let back: RuntimeHandle = proto.into();

            prop_assert_eq!(&back.id, &handle.id);
            prop_assert_eq!(&back.runtime_name, &handle.runtime_name);
        }

        #[test]
        fn runtime_handle_with_data_roundtrip(
            id in "[a-z0-9-]{1,32}",
            runtime_name in "[a-z0-9-]{1,32}",
            key in "[a-z]{1,10}",
            value in "[a-z0-9]{1,20}",
        ) {
            let mut data = HashMap::new();
            data.insert(key.clone(), serde_json::Value::String(value.clone()));

            let handle = RuntimeHandle {
                id,
                runtime_name,
                data,
            };

            let proto: ProtoRuntimeHandle = (&handle).into();
            let back: RuntimeHandle = proto.into();

            prop_assert_eq!(&back.id, &handle.id);
            prop_assert_eq!(&back.runtime_name, &handle.runtime_name);
            let back_val = back.data.get(&key).unwrap();
            prop_assert_eq!(back_val.as_str().unwrap(), &value);
        }

        #[test]
        fn create_runtime_request_preserves_fields(
            session_id_str in "[a-z0-9-]{3,20}",
            launch_command in "[a-z ]{1,50}",
            cwd in "/[a-z/]{1,30}",
            session_name in "[a-z0-9-]{1,20}",
        ) {
            let session_id = SessionId::new(&session_id_str).unwrap();
            let config = RuntimeCreateConfig {
                session_id,
                launch_command: launch_command.clone(),
                env: HashMap::new(),
                cwd: cwd.clone(),
                session_name: session_name.clone(),
            };

            let req: CreateRuntimeRequest = (&config).into();

            prop_assert_eq!(&req.session_id, &session_id_str);
            prop_assert_eq!(&req.launch_command, &launch_command);
            prop_assert_eq!(&req.cwd, &cwd);
            prop_assert_eq!(&req.session_name, &session_name);
            prop_assert!(req.env.is_empty());
        }
    }
}
