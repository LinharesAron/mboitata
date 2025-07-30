use pki_types::pem::{PemObject, SectionKind};
use rcgen::string::Ia5String;
use rcgen::{
    CertificateParams, DistinguishedName, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose, SanType,
};
use rustls::ServerConfig;
use rustls::pki_types::PrivateKeyDer;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

type CertCache = Arc<RwLock<HashMap<String, Arc<ServerConfig>>>>;

#[derive(Debug, Clone)]
pub struct CertificateManager {
    issuer: Arc<Issuer<'static, KeyPair>>,
    cert_cache: CertCache,
}

impl CertificateManager {
    pub fn new(ca_cert_pem: &str, ca_key_pem: &str) -> anyhow::Result<Self> {
        let key = KeyPair::from_pem(ca_key_pem)?;
        let issuer = Issuer::from_ca_cert_pem(ca_cert_pem, key)?;

        Ok(Self {
            issuer: Arc::new(issuer),
            cert_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn get_server_config(&self, hostname: &str) -> anyhow::Result<Arc<ServerConfig>> {
        let hostname = normalize_hostname(hostname);
        if let Some(cfg) = self.cert_cache.read().await.get(hostname.as_str()) {
            return Ok(cfg.clone());
        }

        let server_config = Arc::new(self.generate_certificate_for_host(hostname.as_str())?);

        self.cert_cache
            .write()
            .await
            .insert(hostname.to_string(), server_config.clone());

        Ok(server_config)
    }

    fn generate_certificate_for_host(&self, hostname: &str) -> anyhow::Result<ServerConfig> {
        let mut params = CertificateParams::new(vec![hostname.to_string()]).unwrap();

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, hostname);
        dn.push(DnType::OrganizationName, "Mboi Tata Proxy");
        params.distinguished_name = dn;

        params.subject_alt_names = vec![SanType::DnsName(Ia5String::try_from(hostname)?)];

        let key_pair = KeyPair::generate()?;
        let cert = params.signed_by(&key_pair, &self.issuer)?;

        let private_key =
            PrivateKeyDer::from_pem(SectionKind::PrivateKey, key_pair.serialize_der()).unwrap();

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert.der().clone()], private_key)?;

        Ok(config)
    }
}

pub fn create_ca_certificate() -> anyhow::Result<(String, String)> {
    let mut params = CertificateParams::default();

    params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];

    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "Mboi Tata Proxy Root CA");
    dn.push(DnType::OrganizationName, "Mboi Tata");
    params.distinguished_name = dn;

    let key = KeyPair::generate()?;
    let cert = params.self_signed(&key)?;

    Ok((cert.pem(), key.serialize_pem()))
}

fn normalize_hostname(h: &str) -> String {
    h.split(':')
        .next()
        .unwrap_or(h)
        .trim_end_matches('.')
        .to_ascii_lowercase()
}
