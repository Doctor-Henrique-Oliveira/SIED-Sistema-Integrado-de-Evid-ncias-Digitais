//! Identificadores globais.
//!
//! Casos e dispositivos usam UUID v7 (ordenável por tempo e único entre máquinas).
//! Assim, bancos de casos diferentes podem ser anexados e correlacionados no futuro
//! sem colisão de chaves.

use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;
use uuid::Uuid;

macro_rules! id_uuid {
    ($nome:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS,
        )]
        #[serde(transparent)]
        #[ts(export, type = "string")]
        pub struct $nome(pub Uuid);

        impl $nome {
            pub fn novo() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl fmt::Display for $nome {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $nome {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s).map(Self)
            }
        }
    };
}

id_uuid!(CasoId);
id_uuid!(DispositivoId);
