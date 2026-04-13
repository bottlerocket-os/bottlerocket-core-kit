//! Bottlerocket package inventory to CycloneDX SBOM converter.
//!
//! This module reads the system's application inventory and transforms it into
//! a CycloneDX 1.5 Software Bill of Materials (SBOM) format. The SBOM includes
//! host metadata properties formatted for Amazon Inspector compatibility.

use crate::error::{self, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::fs;
use uuid::Uuid;

/// Host and IMDS metadata collected for SBOM property generation.
///
/// These fields map to Amazon Inspector's expected property naming scheme,
/// enabling vulnerability scanning and asset identification.
#[derive(Default)]
pub struct HostMetadata {
    pub hostname: String,
    pub kernel_name: String,
    pub kernel_version: String,
    pub cpu_architecture: String,
    pub instance_id: String,
    pub instance_type: String,
    pub region: String,
    pub partition: String,
    pub account_id: String,
}

/// A key-value property attached to SBOM components.
#[derive(Serialize)]
pub struct Property {
    pub name: String,
    pub value: String,
}

/// Wrapper for license information in CycloneDX format.
#[derive(Serialize)]
pub struct LicenseEntry {
    pub license: LicenseId,
}

/// SPDX license identifier.
#[derive(Serialize)]
pub struct LicenseId {
    pub id: String,
}

const INVENTORY_PATH: &str = "/usr/share/bottlerocket/application-inventory.json";

const VERSION: &str = "0.1.0";

/// Deserialized application inventory from Bottlerocket's package list.
#[derive(Deserialize)]
pub struct Inventory {
    #[serde(rename = "Content")]
    pub content: Vec<Package>,
}

/// Individual package entry from the inventory file.
#[derive(Deserialize)]
pub struct Package {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Publisher")]
    pub publisher: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Release")]
    pub release: String,
    #[serde(rename = "Epoch")]
    pub epoch: String,
    #[serde(rename = "Architecture")]
    pub architecture: String,
    #[serde(rename = "Url")]
    pub _url: String,
    #[serde(rename = "Summary")]
    pub summary: String,
}

/// Root CycloneDX SBOM document structure.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sbom {
    pub bom_format: &'static str,
    pub spec_version: &'static str,
    /// Unique identifier for this SBOM instance (UUID v4).
    pub serial_number: String,
    pub version: u32,
    pub metadata: Metadata,
    pub components: Vec<Component>,
}

/// SBOM metadata including generation timestamp and tooling info.
#[derive(Serialize)]
pub struct Metadata {
    pub timestamp: String,
    pub tools: Tools,
}

/// Container for tool components that generated this SBOM.
#[derive(Serialize)]
pub struct Tools {
    pub components: Vec<ToolComponent>,
}

/// Describes the tool used to generate the SBOM.
#[derive(Serialize)]
pub struct ToolComponent {
    #[serde(rename = "type")]
    pub typ: &'static str,
    pub author: &'static str,
    pub name: &'static str,
    pub version: &'static str,
}

/// A software component entry in the SBOM.
///
/// The first component is always the OS itself (with host metadata properties),
/// followed by individual packages as library components.
#[derive(Serialize)]
pub struct Component {
    #[serde(rename = "bom-ref")]
    pub bom_ref: String,
    #[serde(rename = "type")]
    pub typ: &'static str,
    pub name: String,
    pub version: String,
    /// Package URL per the purl spec for package identification.
    pub purl: String,
    pub publisher: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub licenses: Option<Vec<LicenseEntry>>,
    /// Properties use the `amazon:inspector:sbom_generator:metadata:` prefix
    /// to conform to Amazon Inspector's expected schema for host/IMDS data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Vec<Property>>,
}

/// Reads the Bottlerocket inventory and converts it to CycloneDX SBOM JSON.
///
/// The SBOM structure:
/// 1. OS component (type: operating-system) with host/IMDS metadata as properties
/// 2. Package components (type: library) with purl identifiers for each installed package
///
/// Property names follow Amazon Inspector's naming convention:
/// `amazon:inspector:sbom_generator:metadata:{category}:{field}`
/// where category is either `host` (uname data) or `imds` (EC2 instance metadata).
pub fn read_and_convert(metadata: &HostMetadata) -> Result<String> {
    let data = fs::read_to_string(INVENTORY_PATH).context(error::ReadInventorySnafu)?;
    let inv: Inventory = serde_json::from_str(&data).context(error::ParseInventorySnafu)?;

    // Extract Bottlerocket version from the bottlerocket-metadata package
    let br_version = inv
        .content
        .iter()
        .find(|p| p.name == "bottlerocket-metadata")
        .map(|p| p.version.as_str())
        .unwrap_or("unknown");

    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    // Build properties using Amazon Inspector's expected naming scheme.
    // The prefix `amazon:inspector:sbom_generator:metadata:` is required for
    // Inspector to recognize and process these fields during vulnerability scans.
    let properties = vec![
        Property {
            name: "amazon:inspector:sbom_generator:metadata:host:hostname".into(),
            value: metadata.hostname.clone(),
        },
        Property {
            name: "amazon:inspector:sbom_generator:metadata:host:kernel_name".into(),
            value: metadata.kernel_name.clone(),
        },
        Property {
            name: "amazon:inspector:sbom_generator:metadata:host:kernel_version".into(),
            value: metadata.kernel_version.clone(),
        },
        Property {
            name: "amazon:inspector:sbom_generator:metadata:host:cpu_architecture".into(),
            value: metadata.cpu_architecture.clone(),
        },
        Property {
            name: "amazon:inspector:sbom_generator:metadata:imds:provider".into(),
            value: "aws".into(),
        },
        Property {
            name: "amazon:inspector:sbom_generator:metadata:imds:instance_id".into(),
            value: metadata.instance_id.clone(),
        },
        Property {
            name: "amazon:inspector:sbom_generator:metadata:imds:instance_type".into(),
            value: metadata.instance_type.clone(),
        },
        Property {
            name: "amazon:inspector:sbom_generator:metadata:imds:instance_location".into(),
            value: metadata.region.clone(),
        },
        Property {
            name: "amazon:inspector:sbom_generator:metadata:imds:instance_partition".into(),
            value: metadata.partition.clone(),
        },
        Property {
            name: "amazon:inspector:sbom_generator:metadata:imds:account_id".into(),
            value: metadata.account_id.clone(),
        },
        Property {
            name: "amazon:inspector:sbom_generator:metadata:imds:resource_type".into(),
            value: "ec2:instance".into(),
        },
    ];

    // OS component carries all host/IMDS metadata as properties
    let os_component = Component {
        bom_ref: "comp-os".into(),
        typ: "operating-system",
        name: "bottlerocket".into(),
        version: br_version.into(),
        purl: String::new(),
        publisher: "Amazon Web Services, Inc. (AWS)".into(),
        description: "Bottlerocket OS".into(),
        licenses: None,
        properties: Some(properties),
    };

    // Package components use purl format for precise identification:
    // pkg:rpm/bottlerocket/{name}@{version}-{release}?arch={arch}&epoch={epoch}&distro={br_version}
    let mut components = vec![os_component];
    components.extend(inv.content.iter().enumerate().map(|(i, p)| Component {
        bom_ref: format!("comp-{}", i),
        typ: "library",
        name: p.name.clone(),
        version: format!("{}-{}", p.version, p.release),
        purl: format!(
            "pkg:rpm/bottlerocket/{}@{}-{}?arch={}&epoch={}&distro={}",
            p.name,
            p.version,
            p.release,
            p.architecture,
            if p.epoch.is_empty() { "0" } else { &p.epoch },
            br_version,
        ),
        publisher: p.publisher.clone(),
        description: p.summary.clone(),
        licenses: Some(vec![LicenseEntry {
            license: LicenseId {
                id: "Apache-2.0 OR MIT".into(),
            },
        }]),
        properties: None,
    }));

    let sbom = Sbom {
        bom_format: "CycloneDX",
        spec_version: "1.5",
        serial_number: format!("urn:uuid:{}", Uuid::new_v4()),
        version: 1,
        metadata: Metadata {
            timestamp,
            tools: Tools {
                components: vec![ToolComponent {
                    typ: "application",
                    author: "Amazon Web Services, Inc. (AWS)",
                    name: "corgid",
                    version: VERSION,
                }],
            },
        },
        components,
    };

    let json = serde_json::to_string_pretty(&sbom).context(error::SerializeSbomSnafu)?;
    Ok(json)
}
