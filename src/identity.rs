use age::secrecy::ExposeSecret;
use age::x25519::Identity as AgeIdentity;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A ReticulumChat identity wrapping an age X25519 identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub private_key: String,
    pub public_key: String,
    pub address: String,
}

impl Identity {
    /// Generate a new random identity
    pub fn generate(name: impl Into<String>) -> Result<Self> {
        let age_identity = AgeIdentity::generate();
        let public_key = age_identity.to_public();

        let private_key = age_identity.to_string().expose_secret().to_string();
        let public_key_str = public_key.to_string();
        let address = public_key_str.clone();

        Ok(Self {
            name: name.into(),
            private_key,
            public_key: public_key_str,
            address,
        })
    }

    /// Load an age identity from the stored string
    pub fn age_identity(&self) -> Result<AgeIdentity> {
        let identity: AgeIdentity = self
            .private_key
            .parse()
            .map_err(|e: &str| anyhow::anyhow!("Failed to parse identity: {}", e))?;
        Ok(identity)
    }

    /// Get the public key as a string
    pub fn public_key_str(&self) -> String {
        self.public_key.clone()
    }

    /// Get the Reticulum destination hash derived from this identity
    pub fn destination_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.address.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

impl fmt::Display for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name, self.address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let identity = Identity::generate("alice").unwrap();
        assert_eq!(identity.name, "alice");
        assert!(!identity.address.is_empty());
        assert!(!identity.destination_hash().is_empty());
    }
}
