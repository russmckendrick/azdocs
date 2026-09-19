//! Azure cloud environments. Every endpoint azdocs talks to hangs off one of
//! these, so a sovereign tenant is a setting rather than a fork.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum Cloud {
    #[default]
    Public,
    #[serde(rename = "usgov")]
    UsGov,
    China,
}

/// The three hosts a cloud differs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloudEndpoints {
    /// Entra authority, without a trailing slash.
    pub authority: &'static str,
    /// ARM base for Resource Graph, RBAC and site management.
    pub arm: &'static str,
    /// OAuth scope for a management token.
    pub scope: &'static str,
}

const PUBLIC: CloudEndpoints = CloudEndpoints {
    authority: "https://login.microsoftonline.com",
    arm: "https://management.azure.com",
    scope: "https://management.azure.com/.default",
};
const USGOV: CloudEndpoints = CloudEndpoints {
    authority: "https://login.microsoftonline.us",
    arm: "https://management.usgovcloudapi.net",
    scope: "https://management.usgovcloudapi.net/.default",
};
const CHINA: CloudEndpoints = CloudEndpoints {
    authority: "https://login.chinacloudapi.cn",
    arm: "https://management.chinacloudapi.cn",
    scope: "https://management.chinacloudapi.cn/.default",
};

impl Cloud {
    pub const ALL: [Cloud; 3] = [Cloud::Public, Cloud::UsGov, Cloud::China];

    pub fn endpoints(self) -> &'static CloudEndpoints {
        match self {
            Self::Public => &PUBLIC,
            Self::UsGov => &USGOV,
            Self::China => &CHINA,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::UsGov => "usgov",
            Self::China => "china",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|cloud| cloud.as_str() == value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_usgov_endpoints_use_usgov_authority() {
        let endpoints = Cloud::UsGov.endpoints();
        assert_eq!(endpoints.authority, "https://login.microsoftonline.us");
        assert!(endpoints.scope.starts_with(endpoints.arm));
    }

    #[test]
    fn unit_cloud_round_trips_through_serde_and_strings() {
        for cloud in Cloud::ALL {
            assert_eq!(Cloud::parse(cloud.as_str()), Some(cloud));
            let json = serde_json::to_string(&cloud).unwrap();
            assert_eq!(json, format!("\"{}\"", cloud.as_str()));
            assert_eq!(serde_json::from_str::<Cloud>(&json).unwrap(), cloud);
        }
        assert!(serde_json::from_str::<Cloud>("\"mars\"").is_err());
    }
}
