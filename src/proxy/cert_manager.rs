//! Certificate manager for MITM proxy
//! Ported from src-tauri/src/proxy/cert_manager.rs

use rcgen::{CertificateParams, KeyPair};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Error, Debug)]
pub enum CertError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Certificate generation error: {0}")]
    Generation(String),
    #[error("TLS error: {0}")]
    Tls(String),
}

pub struct CertificateManager {
    data_dir: PathBuf,
    ca_key: Arc<RwLock<Option<KeyPair>>>,
    ca_cert_pem: Arc<RwLock<Option<String>>>,
}

impl CertificateManager {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            ca_key: Arc::new(RwLock::new(None)),
            ca_cert_pem: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize certificate manager, generating CA if needed
    pub async fn initialize(&self) -> Result<(), CertError> {
        let cert_dir = self.data_dir.join("certificates");
        fs::create_dir_all(&cert_dir)?;

        let ca_cert_path = cert_dir.join("ca.crt");
        let ca_key_path = cert_dir.join("ca.key");

        if ca_cert_path.exists() && ca_key_path.exists() {
            // Load existing CA
            let cert_pem = fs::read_to_string(&ca_cert_path)?;
            let key_pem = fs::read_to_string(&ca_key_path)?;

            // Parse key
            let key_pair = KeyPair::from_pem(&key_pem)
                .map_err(|e| CertError::Generation(e.to_string()))?;

            *self.ca_key.write().await = Some(key_pair);
            *self.ca_cert_pem.write().await = Some(cert_pem);
        } else {
            // Generate new CA
            self.generate_ca().await?;
        }

        Ok(())
    }

    /// Generate a new CA certificate
    async fn generate_ca(&self) -> Result<(), CertError> {
        let mut params = CertificateParams::default();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params.distinguished_name.push(
            rcgen::DnType::CommonName,
            "KGC Toolkit CA",
        );
        params.distinguished_name.push(
            rcgen::DnType::OrganizationName,
            "KGC Toolkit",
        );

        let key_pair = KeyPair::generate()
            .map_err(|e| CertError::Generation(e.to_string()))?;

        let cert = params.self_signed(&key_pair)
            .map_err(|e| CertError::Generation(e.to_string()))?;

        // Save to disk
        let cert_dir = self.data_dir.join("certificates");
        fs::create_dir_all(&cert_dir)?;

        let ca_cert_path = cert_dir.join("ca.crt");
        let ca_key_path = cert_dir.join("ca.key");

        let cert_pem = cert.pem();
        fs::write(&ca_cert_path, &cert_pem)?;
        fs::write(&ca_key_path, key_pair.serialize_pem())?;

        *self.ca_key.write().await = Some(key_pair);
        *self.ca_cert_pem.write().await = Some(cert_pem);

        println!("[CertManager] Generated new CA certificate");

        Ok(())
    }

    /// Get path to CA certificate for download
    pub fn get_ca_cert_path(&self) -> PathBuf {
        self.data_dir.join("certificates").join("ca.crt")
    }

    /// Get CA certificate PEM content
    pub async fn get_ca_cert_pem(&self) -> Option<String> {
        self.ca_cert_pem.read().await.clone()
    }

    /// Generate a certificate for a specific domain
    pub async fn generate_domain_cert(&self, domain: &str) -> Result<(String, String), CertError> {
        let ca_key = self.ca_key.read().await;
        let ca_cert_pem = self.ca_cert_pem.read().await;

        let _ca_key = ca_key.as_ref().ok_or_else(|| {
            CertError::Generation("CA key not initialized".into())
        })?;

        let _ca_cert_pem = ca_cert_pem.as_ref().ok_or_else(|| {
            CertError::Generation("CA cert not initialized".into())
        })?;

        let params = CertificateParams::new(vec![domain.to_string()])
            .map_err(|e| CertError::Generation(e.to_string()))?;

        let domain_key = KeyPair::generate()
            .map_err(|e| CertError::Generation(e.to_string()))?;

        // For now, self-sign domain certs (proper implementation would use CA to sign)
        // rcgen's signed_by requires the CA Certificate, not just PEM
        let cert = params.self_signed(&domain_key)
            .map_err(|e| CertError::Generation(e.to_string()))?;

        Ok((cert.pem(), domain_key.serialize_pem()))
    }
}
