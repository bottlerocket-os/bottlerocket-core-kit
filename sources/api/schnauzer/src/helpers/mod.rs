// This module contains helpers for rendering templates. These helpers can
// be registered with the Handlebars library to assist in manipulating
// text at render time.

use bottlerocket_modeled_types::{OciDefaultsCapability, OciDefaultsResourceLimitType};
use cidr::AnyIpCidr;
use dns_lookup::lookup_host;
use error::TemplateHelperError;
use handlebars::{
    handlebars_helper, Context, Handlebars, Helper, Output, RenderContext, RenderError,
};
use lazy_static::lazy_static;
use serde::Deserialize;
use serde_json::value::Value;
use serde_plain::derive_fromstr_from_deserialize;
use settings_extension_oci_defaults::OciDefaultsResourceLimitV1;
use snafu::{OptionExt, ResultExt};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use url::Url;

pub mod stdlib;

pub use stdlib::{
    any_enabled, base64_decode, default, goarch, join_array, join_map, negate_or_else, toml_encode,
    IfNotNullHelper, IsArray, IsBool, IsNull, IsNumber, IsObject, IsString,
};

lazy_static! {
    /// A map to tell us which registry to pull ECR images from for a given region.
    static ref ECR_MAP: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("af-south-1", "917644944286");
        m.insert("ap-east-1", "375569722642");
        m.insert("ap-northeast-1", "328549459982");
        m.insert("ap-northeast-2", "328549459982");
        m.insert("ap-northeast-3", "328549459982");
        m.insert("ap-south-1", "328549459982");
        m.insert("ap-south-2", "764716012617");
        m.insert("ap-southeast-1", "328549459982");
        m.insert("ap-southeast-2", "328549459982");
        m.insert("ap-southeast-3", "386774335080");
        m.insert("ap-southeast-4", "731751899352");
        m.insert("ap-southeast-5", "851725293737");
        m.insert("ca-central-1", "328549459982");
        m.insert("ca-west-1", "253897149516");
        m.insert("cn-north-1", "183470599484");
        m.insert("cn-northwest-1", "183901325759");
        m.insert("eu-central-1", "328549459982");
        m.insert("eu-central-2", "861738308508");
        m.insert("eu-isoe-west-1", "589460436674");
        m.insert("eu-north-1", "328549459982");
        m.insert("eu-south-1", "586180183710");
        m.insert("eu-south-2", "620625777247");
        m.insert("eu-west-1", "328549459982");
        m.insert("eu-west-2", "328549459982");
        m.insert("eu-west-3", "328549459982");
        m.insert("il-central-1", "288123944683");
        m.insert("me-central-1", "553577323255");
        m.insert("me-south-1", "509306038620");
        m.insert("sa-east-1", "328549459982");
        m.insert("us-east-1", "328549459982");
        m.insert("us-east-2", "328549459982");
        m.insert("us-gov-east-1", "388230364387");
        m.insert("us-gov-west-1", "347163068887");
        m.insert("us-west-1", "328549459982");
        m.insert("us-west-2", "328549459982");
        m
    };
}

/// But if there is a region that does not exist in our map (for example a new
/// region is created or being tested), then we will fallback to pulling ECR
/// containers from here.
const ECR_FALLBACK_REGION: &str = "us-east-1";
const ECR_FALLBACK_REGISTRY: &str = "328549459982";

lazy_static! {
    /// A map to tell us which endpoint to pull updates from for a given region.
    static ref TUF_ENDPOINT_MAP: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("cn-north-1", "bottlerocket-updates-cn-north-1.s3.dualstack");
        m.insert("cn-northwest-1", "bottlerocket-updates-cn-northwest-1.s3.dualstack");
        m.insert("eu-isoe-west-1", "bottlerocket-updates-eu-isoe-west-1.s3");
        m
    };
}

const TUF_PUBLIC_REPOSITORY: &str = "https://updates.bottlerocket.aws";

lazy_static! {
    /// A map to tell us the partition for a given non-standard region.
    static ref ALT_PARTITION_MAP: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("cn-north-1", "aws-cn");
        m.insert("cn-northwest-1", "aws-cn");
        m.insert("eu-isoe-west-1", "aws-iso-e");
        m.insert("us-gov-east-1", "aws-us-gov");
        m.insert("us-gov-west-1", "aws-us-gov");
        m
    };
}

/// The partition for standard AWS regions.
const STANDARD_PARTITION: &str = "aws";

/// The amount of CPU to reserve
/// We are using these CPU ranges from GKE
/// (https://cloud.google.com/kubernetes-engine/docs/concepts/cluster-architecture#node_allocatable):
/// 6% of the first core
/// 1% of the next core (up to 2 cores)
/// 0.5% of the next 2 cores (up to 4 cores)
/// 0.25% of any cores above 4 cores
const KUBE_RESERVE_1_CORE: f32 = 60.0;
const KUBE_RESERVE_2_CORES: f32 = KUBE_RESERVE_1_CORE + 10.0;
const KUBE_RESERVE_3_CORES: f32 = KUBE_RESERVE_2_CORES + 5.0;
const KUBE_RESERVE_4_CORES: f32 = KUBE_RESERVE_3_CORES + 5.0;
const KUBE_RESERVE_ADDITIONAL: f32 = 2.5;

const IPV4_LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const IPV6_LOCALHOST: IpAddr = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));

const DEFAULT_ECS_METADATA_SERVICE_RPS: i32 = 40;
const DEFAULT_ECS_METADATA_SERVICE_BURST: i32 = 60;
/// We use -1 to indicate unlimited value for resource limits.
const RLIMIT_UNLIMITED: i64 = -1;

/// Potential errors during helper execution
mod error {
    use handlebars::RenderError;
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum TemplateHelperError {
        #[snafu(display("Expected an AWS region, got '{}' in template {}", value, template))]
        AwsRegion {
            value: handlebars::JsonValue,
            template: String,
        },

        #[snafu(display(
            "Incorrect number of params provided to helper '{}' in template '{}' - {} expected, {} received",
            helper,
            template,
            expected,
            received,
        ))]
        IncorrectNumberOfParams {
            expected: usize,
            received: usize,
            helper: String,
            template: String,
        },

        #[snafu(display("Internal error: {}", msg))]
        Internal { msg: String },

        #[snafu(display("Internal error: Missing param after confirming that it existed."))]
        ParamUnwrap {},

        #[snafu(display("Invalid OCI spec section: {}", source))]
        InvalidOciSpecSection { source: serde_plain::Error },

        // handlebars::JsonValue is a serde_json::Value, which implements
        // the 'Display' trait and should provide valuable context
        #[snafu(display(
            "Invalid template value, expected {}, got '{}' in template {}",
            expected,
            value,
            template
        ))]
        InvalidTemplateValue {
            expected: &'static str,
            value: handlebars::JsonValue,
            template: String,
        },

        #[snafu(display(
            "Unable to parse template value, expected {}, got '{}' in template {}: '{}'",
            expected,
            value,
            template,
            source,
        ))]
        UnparseableTemplateValue {
            source: serde_json::Error,
            expected: &'static str,
            value: handlebars::JsonValue,
            template: String,
        },

        #[snafu(display(
            "The join_array helper expected type '{}' while processing '{}' for template '{}'",
            expected_type,
            value,
            template
        ))]
        JoinStringsWrongType {
            expected_type: &'static str,
            value: handlebars::JsonValue,
            template: String,
        },

        #[snafu(display("Missing param {} for helper '{}'", index, helper_name))]
        MissingParam { index: usize, helper_name: String },

        #[snafu(display(
            "Missing parameter path for param {} for helper '{}'",
            index,
            helper_name
        ))]
        MissingParamPath { index: usize, helper_name: String },

        #[snafu(display(
            "Missing data and fail-if-missing was set; see given line/col in template '{}'",
            template,
        ))]
        MissingTemplateData { template: String },

        #[snafu(display("Unable to decode base64 in template '{}': '{}'", template, source))]
        Base64Decode {
            template: String,
            source: base64::DecodeError,
        },

        #[snafu(display(
            "Invalid (non-utf8) output from base64 string '{}' in template '{}': '{}'",
            base64_string,
            template,
            source
        ))]
        InvalidUTF8 {
            base64_string: String,
            template: String,
            source: std::str::Utf8Error,
        },

        #[snafu(display("Unable to write template '{}': '{}'", template, source))]
        TemplateWrite {
            template: String,
            source: std::io::Error,
        },

        #[snafu(display("Unknown architecture '{}' given to goarch helper", given))]
        UnknownArch { given: String },

        #[snafu(display("Invalid Cidr format '{}': {}", cidr, source))]
        InvalidCidr {
            cidr: String,
            source: cidr::errors::NetworkParseError,
        },

        #[snafu(display("Invalid IP Address format '{}': {}", ip_address, source))]
        InvalidIPAddress {
            ip_address: String,
            source: std::net::AddrParseError,
        },

        #[snafu(display("Failed to check if EFA device is attached: {}", source))]
        CheckEfaFailure { source: pciclient::PciClientError },

        #[snafu(display(
            "Expected an absolute URL, got '{}' in template '{}': '{}'",
            url_str,
            template,
            source
        ))]
        UrlParse {
            url_str: String,
            template: String,
            source: url::ParseError,
        },

        #[snafu(display("URL '{}' is missing host component", url_str))]
        UrlHost { url_str: String },

        #[snafu(display("Failed to convert {} {} to {}", what, number, target))]
        ConvertNumber {
            what: String,
            number: String,
            target: String,
        },

        #[snafu(display("Failed to convert usize {} to u16: {}", number, source))]
        ConvertUsizeToU16 {
            number: usize,
            source: std::num::TryFromIntError,
        },

        #[snafu(display("Invalid output type '{}', expected 'docker' or 'containerd'", runtime))]
        InvalidOutputType {
            source: serde_plain::Error,
            runtime: String,
        },

        #[snafu(display("Invalid metadata service limits '{},{}'", rps, burst))]
        InvalidMetadataServiceLimits {
            rps: handlebars::JsonValue,
            burst: handlebars::JsonValue,
        },

        #[snafu(display(
            "Unable to encode input '{}' from template '{}' as toml: {}",
            value,
            source,
            template
        ))]
        TomlEncode {
            value: serde_json::Value,
            source: serde_json::Error,
            template: String,
        },
    }

    // Handlebars helpers are required to return a RenderError.
    // Implement "From" for TemplateHelperError.
    impl From<TemplateHelperError> for RenderError {
        fn from(e: TemplateHelperError) -> RenderError {
            RenderError::from_error("TemplateHelperError", e)
        }
    }
}

/// `join_node_taints` is a specialized version of `join_map` that joins the kubernetes node taints
/// setting in the correct format `kubelet` expects for its `--register-with-taints` option.
///
/// Example:
///    {{ join_node_taints settings.kubernetes.node-taints }}
///    ...where `settings.kubernetes.node-taints` is: {"key1": ["value1:NoSchedule","value1:NoExecute"], "key2": ["value2:NoSchedule"]}
///    ...will produce: "key1=value1:NoSchedule,key1=value1:NoExecute,key2=value2:NoSchedule"
pub fn join_node_taints(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    trace!("Starting join_node_taints helper");
    let template_name = template_name(renderctx);
    trace!("Template name: {}", &template_name);

    trace!("Number of params: {}", helper.params().len());
    check_param_count(helper, template_name, 1)?;

    let node_taints_value = get_param(helper, 0)?;
    // It's ok if there are no node-taints, output nothing
    if !node_taints_value.is_object() {
        return Ok(());
    }

    let node_taints = node_taints_value
        .as_object()
        .context(error::InternalSnafu {
            msg: "Already confirmed map is_object but as_object failed",
        })?;
    trace!("node taints to join: {:?}", node_taints);

    // Join the key/value pairs for node taints
    let mut pairs = Vec::new();
    for (key, val_value) in node_taints.into_iter() {
        match val_value {
            Value::Array(values) => {
                for taint_value in values {
                    if let Some(taint_str) = taint_value.as_str() {
                        pairs.push(format!("{}={}", key, taint_str));
                    } else {
                        return Err(RenderError::from(
                            error::TemplateHelperError::InvalidTemplateValue {
                                expected: "string",
                                value: taint_value.to_owned(),
                                template: template_name.to_owned(),
                            },
                        ));
                    }
                }
            }
            Value::Null => {
                return Err(RenderError::from(
                    error::TemplateHelperError::InvalidTemplateValue {
                        expected: "non-null",
                        value: val_value.to_owned(),
                        template: template_name.to_owned(),
                    },
                ))
            }
            // all other types unsupported
            _ => {
                return Err(RenderError::from(
                    error::TemplateHelperError::InvalidTemplateValue {
                        expected: "sequence",
                        value: val_value.to_owned(),
                        template: template_name.to_owned(),
                    },
                ))
            }
        };
    }

    // Join all pairs with the given string.
    let joined = pairs.join(",");
    trace!("Joined output: {}", joined);

    // Write the string out to the template
    out.write(&joined).context(error::TemplateWriteSnafu {
        template: template_name.to_owned(),
    })?;

    Ok(())
}

/// The `ecr-prefix` helper is used to map an AWS region to the correct ECR
/// registry.
///
/// Initially we held all of our ECR repos in a single registry, but with some
/// regions this was no longer possible. Because the ECR repo URL includes the
/// the registry number, we created this helper to lookup the correct registry
/// number for a given region.
///
/// This helper takes the AWS region as its only parameter, and returns the
/// fully qualified domain name to the correct ECR registry.
///
/// # Fallback
///
/// A map of region to ECR registry ID is maintained herein. But if we do not
/// have the region in our map, a fallback region and registry number are
/// returned. This would allow a version of Bottlerocket to run in a new region
/// before this map has been updated.
///
/// # Example
///
/// In this example the registry number for the region will be returned.
/// `{{ ecr-prefix settings.aws.region }}`
///
/// This would result in something like:
/// `328549459982.dkr.ecr.eu-central-1.amazonaws.com`
pub fn ecr_prefix(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    trace!("Starting ecr helper");
    let template_name = template_name(renderctx);
    check_param_count(helper, template_name, 1)?;

    // get the region parameter, which is probably given by the template value
    // settings.aws.region. regardless, we expect it to be a string.
    let aws_region = get_param(helper, 0)?;
    let aws_region = aws_region.as_str().with_context(|| error::AwsRegionSnafu {
        value: aws_region.to_owned(),
        template: template_name,
    })?;

    // construct the registry fqdn
    let ecr_registry = ecr_registry(aws_region);

    // write it to the template
    out.write(&ecr_registry)
        .with_context(|_| error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

/// The `tuf-prefix` helper is used to map an AWS region to the correct TUF
/// repository.
///
/// This helper takes the AWS region as its only parameter, and returns the
/// fully qualified domain name to the correct TUF repository.
///
/// # Fallback
///
/// A map of region to TUF repository endpoint is maintained herein. But if we
/// do not have the region in our map, a fallback repository is returned. This
/// would allow a version of Bottlerocket to run in a new region before this map
/// has been updated.
///
/// # Example
///
/// In this example the repository endpoint for the region will be returned.
/// `{{ tuf-prefix settings.aws.region }}`
///
/// This would result in something like:
/// `https://bottlerocket-updates-us-west-2.s3.dualstack.us-west-2.amazonaws.com/latest`
pub fn tuf_prefix(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    trace!("Starting tuf helper");
    let template_name = template_name(renderctx);
    check_param_count(helper, template_name, 1)?;

    // get the region parameter, which is probably given by the template value
    // settings.aws.region. regardless, we expect it to be a string.
    let aws_region = get_param(helper, 0)?;
    let aws_region = aws_region.as_str().with_context(|| error::AwsRegionSnafu {
        value: aws_region.to_owned(),
        template: template_name,
    })?;

    // construct the registry fqdn
    let tuf_repository = tuf_repository(aws_region);

    // write it to the template
    out.write(&tuf_repository)
        .with_context(|_| error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

/// The `metadata-prefix` helper is used to map an AWS region to the correct
/// metadata location inside of the TUF repository.
///
/// This helper takes the AWS region as its only parameter, and returns the
/// prefix of the metadata.
///
pub fn metadata_prefix(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    trace!("Starting tuf helper");
    let template_name = template_name(renderctx);
    check_param_count(helper, template_name, 1)?;

    // get the region parameter, which is probably given by the template value
    // settings.aws.region. regardless, we expect it to be a string.
    let aws_region = get_param(helper, 0)?;
    let aws_region = aws_region.as_str().with_context(|| error::AwsRegionSnafu {
        value: aws_region.to_owned(),
        template: template_name,
    })?;

    // construct the prefix
    let metadata_location = if TUF_ENDPOINT_MAP.contains_key(aws_region) {
        "/metadata"
    } else {
        ""
    };

    // write it to the template
    out.write(metadata_location)
        .with_context(|_| error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

/// `host` takes an absolute URL and trims it down and returns its host.
pub fn host(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    trace!("Starting host helper");
    let template_name = template_name(renderctx);
    check_param_count(helper, template_name, 1)?;

    let url_val = get_param(helper, 0)?;
    let url_str = url_val
        .as_str()
        .with_context(|| error::InvalidTemplateValueSnafu {
            expected: "string",
            value: url_val.to_owned(),
            template: template_name.to_owned(),
        })?;
    let url = Url::parse(url_str).context(error::UrlParseSnafu {
        url_str,
        template: template_name,
    })?;
    let url_host = url.host_str().context(error::UrlHostSnafu { url_str })?;

    // write it to the template
    out.write(url_host)
        .with_context(|_| error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

handlebars_helper!(is_ipv4: |cidr: str| {
    let ip = validate_and_parse_cidr(cidr)?;

    matches!(ip, IpAddr::V4(_))
});

handlebars_helper!(is_ipv6: |cidr: str| {
    let ip = validate_and_parse_cidr(cidr)?;

    matches!(ip, IpAddr::V6(_))
});

/// Converts a CIDR to its corresponding IP address part.
pub fn cidr_to_ipaddr(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    trace!("Starting cidr_to_ipaddr helper");
    let template_name = template_name(renderctx);
    check_param_count(helper, template_name, 1)?;

    let cidr_param = get_param(helper, 0)?;
    let cidr = cidr_param
        .as_str()
        .with_context(|| error::InvalidTemplateValueSnafu {
            expected: "string",
            value: cidr_param.to_owned(),
            template: template_name,
        })?;

    let ip = validate_and_parse_cidr(cidr)?;
    out.write(&ip.to_string())
        .with_context(|_| error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

/// Replaces a specific octet in an IPv4 address with a new value. The octet is specified by index (0-3).
pub fn replace_ipv4_octet(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    trace!("Starting replace_ipv4_octet helper");
    let template_name = template_name(renderctx);
    check_param_count(helper, template_name, 3)?;

    let ip_param = get_param(helper, 0)?;
    let octet_index_param = get_param(helper, 1)?;
    let new_value_param = get_param(helper, 2)?;

    let ip_str = ip_param
        .as_str()
        .with_context(|| error::InvalidTemplateValueSnafu {
            expected: "string",
            value: ip_param.to_owned(),
            template: template_name,
        })?;
    let octet_index =
        octet_index_param
            .as_u64()
            .with_context(|| error::InvalidTemplateValueSnafu {
                expected: "integer",
                value: octet_index_param.to_owned(),
                template: template_name,
            })? as usize;
    let new_value = new_value_param
        .as_str()
        .with_context(|| error::InvalidTemplateValueSnafu {
            expected: "string",
            value: new_value_param.to_owned(),
            template: template_name,
        })?;

    let ip = Ipv4Addr::from_str(ip_str).map_err(|error| {
        RenderError::from(error::TemplateHelperError::InvalidIPAddress {
            ip_address: ip_str.to_string(),
            source: error,
        })
    })?;

    let mut octets = ip.octets();
    if octet_index > 3 {
        return Err(RenderError::new(format!(
            "Invalid index {} for IP address {}",
            octet_index, ip_str
        )));
    }

    octets[octet_index] = new_value
        .parse::<u8>()
        .map_err(|_| RenderError::new("Failed to parse the new octet value as an integer"))?;

    let new_ip = Ipv4Addr::from(octets).to_string();
    out.write(&new_ip)
        .with_context(|_| error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

fn validate_and_parse_cidr(cidr: &str) -> Result<IpAddr, RenderError> {
    let parsed_cidr: AnyIpCidr = cidr.parse().context(error::InvalidCidrSnafu {
        cidr: cidr.to_string(),
    })?;

    let ip_addr = parsed_cidr
        .first_address()
        .ok_or_else(|| RenderError::new("Empty CIDR block"))?;
    Ok(ip_addr)
}

/// kube_reserve_memory and kube_reserve_cpu are taken from EKS' calculations.
/// https://github.com/awslabs/amazon-eks-ami/blob/db28da15d2b696bc08ac3aacc9675694f4a69933/files/bootstrap.sh

/// Calculates the amount of memory to reserve for kubeReserved in mebibytes.
/// Formula: memory_to_reserve = max_num_pods * 11 + 255 is taken from
/// https://github.com/awslabs/amazon-eks-ami/pull/419#issuecomment-609985305
pub fn kube_reserve_memory(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    trace!("Starting kube_reserve_memory helper");
    let template_name = template_name(renderctx);
    trace!("Template name: {}", &template_name);

    trace!("Number of params: {}", helper.params().len());
    check_param_count(helper, template_name, 2)?;

    let max_num_pods_val = get_param(helper, 0)?;
    let max_num_pods = match max_num_pods_val {
        Value::Number(n) => n,

        _ => {
            return Err(RenderError::from(
                error::TemplateHelperError::InvalidTemplateValue {
                    expected: "number",
                    value: max_num_pods_val.to_owned(),
                    template: template_name.to_owned(),
                },
            ))
        }
    };
    let max_num_pods = max_num_pods
        .as_u64()
        .with_context(|| error::ConvertNumberSnafu {
            what: "number of pods",
            number: max_num_pods.to_string(),
            target: "u64",
        })?;

    // Calculates the amount of memory to reserve
    let memory_to_reserve_value = get_param(helper, 1)?;
    let memory_to_reserve = match memory_to_reserve_value {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.to_string(),
        // If no value is set, use the given default.
        Value::Null => {
            format!("{}Mi", (max_num_pods * 11 + 255))
        }
        // composite types unsupported
        _ => {
            return Err(RenderError::from(
                error::TemplateHelperError::InvalidTemplateValue {
                    expected: "scalar",
                    value: memory_to_reserve_value.to_owned(),
                    template: template_name.to_owned(),
                },
            ))
        }
    };

    // write it to the template
    out.write(&memory_to_reserve)
        .with_context(|_| error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

/// Get the amount of CPU to reserve for kubeReserved in millicores
pub fn kube_reserve_cpu(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    trace!("Starting kube_reserve_cpu helper");
    let template_name = template_name(renderctx);
    trace!("Template name: {}", &template_name);

    trace!("Number of params: {}", helper.params().len());
    check_param_count(helper, template_name, 1)?;

    // Calculates the amount of CPU to reserve
    let num_cores = num_cpus::get();
    let cpu_to_reserve_value = get_param(helper, 0)?;
    let cpu_to_reserve = match cpu_to_reserve_value {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.to_string(),
        // If no value is set, use the given default.
        Value::Null => kube_cpu_helper(num_cores)?,
        // composite types unsupported
        _ => {
            return Err(RenderError::from(
                error::TemplateHelperError::InvalidTemplateValue {
                    expected: "scalar",
                    value: cpu_to_reserve_value.to_owned(),
                    template: template_name.to_owned(),
                },
            ))
        }
    };

    // write it to the template
    out.write(&cpu_to_reserve)
        .with_context(|_| error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

/// Completes `localhost` alias lines in /etc/hosts by returning a series of space-delimited host aliases.
///
/// This helper reconciles `settings.network.hostname` and `settings.network.hosts` references to loopback.
/// * `hostname`: Attempts to resolve the current configured hostname in DNS. If unsuccessful, the return
///   includes an alias for the hostname to be included for the given IP version.
/// * `hosts`: For any static `/etc/hosts` mappings which refer to loopback, this includes aliases in the
///   same order specified in `settings.network.hosts`. These settings take the lowest precedence for
///   loopback aliases.
pub fn localhost_aliases(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    // To give context to our errors, get the template name, if available.
    trace!("Starting localhost_aliases helper");
    let template_name = template_name(renderctx);
    trace!("Template name: {}", &template_name);

    // Check number of parameters, must be exactly three (IP version, hostname, hosts overrides)
    trace!("Number of params: {}", helper.params().len());
    check_param_count(helper, template_name, 3)?;

    // Get the resolved keys out of the template. value() returns a serde_json::Value
    let ip_version_value = helper
        .param(0)
        .map(|v| v.value())
        .context(error::ParamUnwrapSnafu {})?;
    trace!("IP version value from template: {}", ip_version_value);

    let hostname_value = helper
        .param(1)
        .map(|v| v.value())
        .context(error::ParamUnwrapSnafu {})?;
    trace!("Hostname value from template: {}", hostname_value);

    let hosts_value = helper
        .param(2)
        .map(|v| v.value())
        .context(error::ParamUnwrapSnafu {})?;
    trace!("Hosts value from template: {}", hosts_value);

    // Extract our variables from their serde_json::Value objects
    let ip_version = ip_version_value
        .as_str()
        .context(error::InvalidTemplateValueSnafu {
            expected: "string",
            value: ip_version_value.to_owned(),
            template: template_name.to_owned(),
        })?;
    trace!("IP version string from template: {}", ip_version);

    let localhost_comparator = match ip_version {
        "ipv4" => IPV4_LOCALHOST,
        "ipv6" => IPV6_LOCALHOST,
        _ => {
            return Err(error::TemplateHelperError::InvalidTemplateValue {
                expected: r#"one of ("ipv4", "ipv6")"#,
                value: ip_version_value.to_owned(),
                template: template_name.to_owned(),
            }
            .into());
        }
    };

    let hostname = hostname_value
        .as_str()
        .context(error::InvalidTemplateValueSnafu {
            expected: "string",
            value: hostname_value.to_owned(),
            template: template_name.to_owned(),
        })?;
    trace!("Hostname string from template: {}", hostname);

    let mut results: Vec<String> = vec![];

    let hosts: Option<bottlerocket_modeled_types::EtcHostsEntries> = (!hosts_value.is_null())
        .then(|| {
            serde_json::from_value(hosts_value.clone()).context(
                error::UnparseableTemplateValueSnafu {
                    expected: "EtcHostsEntries",
                    value: hosts_value.to_owned(),
                    template: template_name.to_owned(),
                },
            )
        })
        .transpose()?;
    trace!("Hosts from template: {:?}", hosts);

    // If our hostname isn't resolveable, add it to the alias list.
    if !hostname.is_empty() && !hostname_resolveable(hostname, hosts.as_ref()) {
        results.push(hostname.to_owned());
    }

    // If hosts are specified and any overrides exist for loopback, add them.
    if let Some(hosts) = hosts {
        // If any static mappings in `settings.network.hosts` are for localhost, add them as well.
        if let Some((_, aliases)) = hosts
            .iter_merged()
            .find(|(ip_address, _)| *ip_address == localhost_comparator)
        {
            // Downcast our hostnames into Strings and append to the results
            let mut hostname_aliases: Vec<String> = aliases
                .into_iter()
                .map(|a| a.as_ref().to_string())
                .collect();
            results.append(&mut hostname_aliases);
        }
    }

    // Write out our localhost aliases.
    let localhost_aliases = results.join(" ");
    out.write(&localhost_aliases)
        .context(error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

/// This helper writes out /etc/hosts lines based on `network.settings.hosts`.
///
/// The map of <IpAddr => Vec<HostAlias>> is written as newline-delimited text lines.
/// Any entries which reference localhost are ignored, as these are intended to be merged
/// with the existing localhost entries via `localhost_aliases`.
pub fn etc_hosts_entries(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    // To give context to our errors, get the template name, if available.
    trace!("Starting etc_hosts_entries helper");
    let template_name = template_name(renderctx);
    trace!("Template name: {}", &template_name);

    // Check number of parameters, must be exactly one (hosts overrides)
    trace!("Number of params: {}", helper.params().len());
    check_param_count(helper, template_name, 1)?;

    // Get the resolved keys out of the template. value() returns a serde_json::Value
    let hosts_value = helper
        .param(0)
        .map(|v| v.value())
        .context(error::ParamUnwrapSnafu {})?;
    trace!("Hosts value from template: {}", hosts_value);

    if hosts_value.is_null() {
        // If hosts aren't set, just exit.
        return Ok(());
    }
    // Otherwise we need to generate /etc/hosts lines, ignoring loopback.
    let mut result_lines: Vec<String> = Vec::new();

    let hosts: bottlerocket_modeled_types::EtcHostsEntries = serde_json::from_value(
        hosts_value.clone(),
    )
    .context(error::UnparseableTemplateValueSnafu {
        expected: "EtcHostsEntries",
        value: hosts_value.to_owned(),
        template: template_name.to_owned(),
    })?;
    trace!("Hosts from template: {:?}", hosts);

    hosts
        .iter_merged()
        .filter(|(ip_address, _)| {
            // Localhost aliases are handled by the `localhost_aliases` helper, so we disregard them here.
            *ip_address != IPV4_LOCALHOST && *ip_address != IPV6_LOCALHOST
        })
        .for_each(|(ip_address, aliases)| {
            // Downcast hostnames to Strings and render the /etc/hosts line.
            let alias_strs: Vec<String> = aliases.iter().map(|a| a.as_ref().into()).collect();

            result_lines.push(format!("{} {}", ip_address, alias_strs.join(" ")));
        });

    out.write(&result_lines.join("\n"))
        .context(error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

/// This helper returns the valid values for the ECS metadata service limits
///
/// This helper returns the properly formatted ECS metadata service limits. When either RPS, burst
/// or both are missing, the default values in the ECS agent are used instead.
pub fn ecs_metadata_service_limits(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    // To give context to our errors, get the template name, if available.
    trace!("Starting ecs_metadata_service_limits helper");
    let template_name = template_name(renderctx);
    trace!("Template name: {}", &template_name);

    // Check number of parameters, must be exactly two (metadata_service_rps and
    // metadata_service_burst)
    trace!("Number of params: {}", helper.params().len());
    check_param_count(helper, template_name, 2)?;

    let metadata_service_rps = helper
        .param(0)
        .map(|v| v.value())
        .context(error::ParamUnwrapSnafu {})?;

    let metadata_service_burst = helper
        .param(1)
        .map(|v| v.value())
        .context(error::ParamUnwrapSnafu {})?;

    let output = match (metadata_service_rps, metadata_service_burst) {
        (Value::Number(rps), Value::Number(burst)) => {
            format!("{},{}", rps, burst)
        }
        (Value::Number(rps), Value::Null) => {
            format!("{},{}", rps, DEFAULT_ECS_METADATA_SERVICE_BURST)
        }
        (Value::Null, Value::Number(burst)) => {
            format!("{},{}", DEFAULT_ECS_METADATA_SERVICE_RPS, burst)
        }
        (Value::Null, Value::Null) => format!(
            "{},{}",
            DEFAULT_ECS_METADATA_SERVICE_RPS, DEFAULT_ECS_METADATA_SERVICE_BURST
        ),
        (rps, burst) => {
            return Err(RenderError::from(
                error::TemplateHelperError::InvalidMetadataServiceLimits {
                    rps: rps.to_owned(),
                    burst: burst.to_owned(),
                },
            ))
        }
    };

    out.write(&output).context(error::TemplateWriteSnafu {
        template: template_name.to_owned(),
    })?;

    Ok(())
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum OciSpecSection {
    Capabilities,
    ResourceLimits,
}

derive_fromstr_from_deserialize!(OciSpecSection);

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum Runtime {
    Docker,
    Containerd,
}

derive_fromstr_from_deserialize!(Runtime);

impl Runtime {
    fn get_capabilities(&self, caps: String) -> String {
        match self {
            Self::Docker => Docker::get_capabilities(caps),
            Self::Containerd => Containerd::get_capabilities(caps),
        }
    }

    fn get_resource_limits(
        &self,
        rlimit_type: &OciDefaultsResourceLimitType,
        values: &OciDefaultsResourceLimitV1,
    ) -> String {
        match self {
            Self::Docker => Docker::get_resource_limits(rlimit_type, values),
            Self::Containerd => Containerd::get_resource_limits(rlimit_type, values),
        }
    }
}

struct Docker;
struct Containerd;

impl Docker {
    /// Formats capabilities for Docker
    fn get_capabilities(caps: String) -> String {
        format!(concat!(r#"["#, "{capabilities}", "]",), capabilities = caps)
    }

    /// Formats resource limits for Docker
    fn get_resource_limits(
        rlimit_type: &OciDefaultsResourceLimitType,
        values: &OciDefaultsResourceLimitV1,
    ) -> String {
        format!(
            r#" "{}":{{ "Name": "{}", "Hard": {}, "Soft": {} }}"#,
            rlimit_type
                .to_linux_string()
                .replace("RLIMIT_", "")
                .to_lowercase(),
            rlimit_type
                .to_linux_string()
                .replace("RLIMIT_", "")
                .to_lowercase(),
            values.hard_limit,
            values.soft_limit,
        )
    }
}

impl Containerd {
    /// Formats capabilities for Containerd
    fn get_capabilities(caps: String) -> String {
        format!(
            concat!(
                r#""bounding": ["#,
                "{capabilities_bounding}",
                "],\n",
                r#""effective": ["#,
                "{capabilities_effective}",
                "],\n",
                r#""permitted": ["#,
                "{capabilities_permitted}",
                "]\n",
            ),
            capabilities_bounding = caps,
            capabilities_effective = caps,
            capabilities_permitted = caps,
        )
    }

    /// Formats resource limits for Containerd
    fn get_resource_limits(
        rlimit_type: &OciDefaultsResourceLimitType,
        values: &OciDefaultsResourceLimitV1,
    ) -> String {
        format!(
            r#"{{ "type": "{}", "hard": {}, "soft": {} }}"#,
            rlimit_type.to_linux_string(),
            Self::get_limit(values.hard_limit),
            Self::get_limit(values.soft_limit),
        )
    }

    /// Converts I64 values to u64 for Containerd
    fn get_limit(limit: i64) -> u64 {
        match limit {
            -1 => u64::MAX,
            _ => limit as u64,
        }
    }
}

/// This helper writes out the default OCI runtime spec.
///
/// The calling pattern is `{{ oci_defaults settings.oci-defaults.resource-limits }}`,
/// where `settings.oci-defaults.resource-limits` is a map of OCI defaults
/// settings and their values, as defined in the model.
/// See the following file for more details:
/// * `sources/models/src/modeled_types/oci_defaults.rs`
///
/// This helper function extracts the setting name from the settings map
/// parameter. Specific setting names are expected, or else this helper
/// will return an error. The specific setting names are defined in the
/// following file:
/// * `sources/models/src/lib.rs`
pub fn oci_defaults(
    helper: &Helper<'_, '_>,
    _: &Handlebars,
    _: &Context,
    renderctx: &mut RenderContext<'_, '_>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    // To give context to our errors, get the template name (e.g. what file we are rendering), if available.
    debug!("Starting oci_defaults helper");
    let template_name = template_name(renderctx);
    debug!("Template name: {}", &template_name);

    // Check number of parameters, must be exactly two (OCI spec section to render and settings values for the section)
    debug!("Number of params: {}", helper.params().len());
    check_param_count(helper, template_name, 2)?;
    debug!("params: {:?}", helper.params());

    debug!("Getting the requested output type to render");
    let runtime_val = get_param(helper, 0)?;
    let runtime_str = runtime_val
        .as_str()
        .with_context(|| error::InvalidTemplateValueSnafu {
            expected: "string",
            value: runtime_val.to_owned(),
            template: template_name.to_owned(),
        })?;

    let runtime = Runtime::from_str(runtime_str).context(error::InvalidOutputTypeSnafu {
        runtime: runtime_str.to_owned(),
    })?;

    debug!("Getting the requested OCI spec section to render");
    let oci_defaults_values = get_param(helper, 1)?;
    // We want the settings path so we know which OCI spec section we are rendering.
    // e.g. settings.oci-defaults.resource-limits
    let settings_path = get_param_key_name(helper, 1)?;
    // Extract the last part of the settings path, which is the OCI spec section we want to render.
    let oci_spec_section = settings_path
        .split('.')
        .last()
        .expect("did not find (got None for) an oci_spec_section");

    // Render the requested OCI spec section
    let section =
        OciSpecSection::from_str(oci_spec_section).context(error::InvalidOciSpecSectionSnafu)?;
    let result_lines = match section {
        OciSpecSection::Capabilities => {
            let capabilities = oci_spec_capabilities(oci_defaults_values)?;
            runtime.get_capabilities(capabilities)
        }
        OciSpecSection::ResourceLimits => {
            let rlimits = generate_oci_resource_limits(oci_defaults_values, EfaLspciDetector {})?;
            rlimits
                .iter()
                .map(|(rlimit_type, values)| runtime.get_resource_limits(rlimit_type, values))
                .collect::<Vec<String>>()
                .join(",\n")
        }
    };

    debug!("{}_section: \n{}", oci_spec_section, result_lines);

    // Write out the final values to the configuration file
    out.write(result_lines.as_str())
        .context(error::TemplateWriteSnafu {
            template: template_name.to_owned(),
        })?;

    Ok(())
}

/// This helper writes out the capabilities section of the default
/// OCI runtime spec that `containerd` will use.
///
/// This function is called by the `oci_defaults` helper function,
/// specifically when matching over the `oci_spec_section` parameter
/// to determine which section of the OCI spec to render.
///
/// This helper function generates the linux capabilities section of
/// the OCI runtime spec from the provided `value` parameter, which is
/// the settings data from the datastore (`settings.oci-defaults.capabilities`).
fn oci_spec_capabilities(value: &Value) -> Result<String, RenderError> {
    let oci_default_capabilities: HashMap<OciDefaultsCapability, bool> =
        serde_json::from_value(value.clone())?;

    // Output the capabilities that are enabled
    let mut capabilities_lines: Vec<String> = oci_default_capabilities
        .iter()
        .filter(|(_, &capability_enabled)| capability_enabled)
        .map(|(&capability, _)| format!("\"{}\"", capability.to_linux_string()))
        .collect();

    // Sort for consistency for human-readers of the OCI spec defaults file.
    capabilities_lines.sort();
    let capabilities_lines_joined = capabilities_lines.join(",\n");

    Ok(capabilities_lines_joined)
}

/// This helper writes out the resource limits section of the default
/// OCI runtime spec that `containerd` will use.
///
/// This function is called by the `oci_defaults` helper function,
/// specifically when matching over the `oci_spec_section` parameter
/// to determine which section of the OCI spec to render.
///
/// This helper function generates the resource limits section of
/// the OCI runtime spec from the provided `value` parameter, which is
/// the settings data from the datastore (`settings.oci-defaults.resource-limits`).
fn generate_oci_resource_limits<T: EfaDetector>(
    value: &Value,
    efa_detector: T,
) -> Result<HashMap<OciDefaultsResourceLimitType, OciDefaultsResourceLimitV1>, RenderError> {
    let mut rlimits = oci_spec_resource_limits(value)?;
    if efa_detector.is_efa_attached()? {
        // We need to increase the locked memory limits from the default 8096KB to unlimited
        // to account for hugepages allocation.
        rlimits
            .entry(OciDefaultsResourceLimitType::MaxLockedMemory)
            .or_insert(OciDefaultsResourceLimitV1 {
                soft_limit: RLIMIT_UNLIMITED,
                hard_limit: RLIMIT_UNLIMITED,
            });
    }
    Ok(rlimits)
}

fn oci_spec_resource_limits(
    value: &Value,
) -> Result<HashMap<OciDefaultsResourceLimitType, OciDefaultsResourceLimitV1>, RenderError> {
    Ok(serde_json::from_value(value.clone())?)
}

trait EfaDetector {
    fn is_efa_attached(&self) -> Result<bool, TemplateHelperError>;
}

struct EfaLspciDetector;

impl EfaDetector for EfaLspciDetector {
    fn is_efa_attached(&self) -> Result<bool, TemplateHelperError> {
        pciclient::is_efa_attached().context(error::CheckEfaFailureSnafu)
    }
}

// =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=
// helpers to the helpers

/// Gets the key name (path) of the parameter at `index`. Returns an error if the param cannot be extracted.
fn get_param_key_name<'a>(
    helper: &'a Helper<'_, '_>,
    index: usize,
) -> Result<&'a String, RenderError> {
    Ok(helper
        .params()
        .get(index)
        .context(error::MissingParamSnafu {
            index,
            helper_name: helper.name(),
        })?
        .relative_path()
        .context(error::MissingParamPathSnafu {
            index,
            helper_name: helper.name(),
        })?)
}

/// Gets the value at `idx` and unwraps it. Returns an error if the param cannot be unwrapped.
fn get_param<'a>(helper: &'a Helper<'_, '_>, idx: usize) -> Result<&'a Value, RenderError> {
    Ok(helper
        .param(idx)
        .map(|v| v.value())
        .context(error::MissingParamSnafu {
            index: idx,
            helper_name: helper.name(),
        })?)
}

/// Get the template name if there is one, otherwise return "dynamic template"
fn template_name<'a>(renderctx: &'a RenderContext<'_, '_>) -> &'a str {
    match renderctx.get_root_template_name() {
        Some(s) => s.as_str(),
        None => "dynamic template",
    }
}

/// Creates a an `IncorrectNumberofParams` error if the number of `helper`
/// params does not equal `expected`. Template name is only used in constructing
/// the error message.
fn check_param_count<S: AsRef<str>>(
    helper: &Helper<'_, '_>,
    template_name: S,
    expected: usize,
) -> Result<(), RenderError> {
    if helper.params().len() != expected {
        return Err(RenderError::from(
            error::TemplateHelperError::IncorrectNumberOfParams {
                expected,
                received: helper.params().len(),
                helper: helper.name().to_string(),
                template: template_name.as_ref().into(),
            },
        ));
    }
    Ok(())
}

/// Constructs the fully qualified domain name for the ECR registry for the
/// given region. Returns a default ECR registry if the region is not mapped.
fn ecr_registry<S: AsRef<str>>(region: S) -> String {
    // lookup the ecr registry ID or fallback to the default region and id
    let (region, registry_id) = match ECR_MAP.borrow().get(region.as_ref()) {
        None => (ECR_FALLBACK_REGION, ECR_FALLBACK_REGISTRY),
        Some(registry_id) => (region.as_ref(), *registry_id),
    };
    let partition = match ALT_PARTITION_MAP.borrow().get(region) {
        None => STANDARD_PARTITION,
        Some(partition) => *partition,
    };
    match partition {
        "aws-cn" => format!("{}.dkr.ecr.{}.amazonaws.com.cn", registry_id, region),
        "aws-iso-e" => format!("{}.dkr.ecr.{}.cloud.adc-e.uk", registry_id, region),
        _ => format!("{}.dkr.ecr.{}.amazonaws.com", registry_id, region),
    }
}

/// Constructs the fully qualified domain name for the TUF repository for the
/// given region. Returns a default if the region is not mapped.
fn tuf_repository<S: AsRef<str>>(region: S) -> String {
    // lookup the repository endpoint or fallback to the public url
    let (region, endpoint) = match TUF_ENDPOINT_MAP.borrow().get(region.as_ref()) {
        None => return TUF_PUBLIC_REPOSITORY.to_string(),
        Some(endpoint) => (region.as_ref(), *endpoint),
    };
    let partition = match ALT_PARTITION_MAP.borrow().get(region) {
        None => STANDARD_PARTITION,
        Some(partition) => *partition,
    };
    match partition {
        "aws-cn" => format!("https://{}.{}.amazonaws.com.cn/latest", endpoint, region),
        "aws-iso-e" => format!("https://{}.{}.cloud.adc-e.uk/latest", endpoint, region),
        _ => format!("https://{}.{}.amazonaws.com/latest", endpoint, region),
    }
}

/// Calculates and returns the amount of CPU to reserve
fn kube_cpu_helper(num_cores: usize) -> Result<String, TemplateHelperError> {
    let num_cores =
        u16::try_from(num_cores).context(error::ConvertUsizeToU16Snafu { number: num_cores })?;
    let millicores_unit = "m";
    let cpu_to_reserve = match num_cores {
        0 => 0.0,
        1 => KUBE_RESERVE_1_CORE,
        2 => KUBE_RESERVE_2_CORES,
        3 => KUBE_RESERVE_3_CORES,
        4 => KUBE_RESERVE_4_CORES,
        _ => {
            let num_cores = f32::from(num_cores);
            KUBE_RESERVE_4_CORES + ((num_cores - 4.0) * KUBE_RESERVE_ADDITIONAL)
        }
    };
    Ok(format!("{}{}", cpu_to_reserve.floor(), millicores_unit))
}

/// Returns whether or not a hostname resolves to a non-loopback IP address.
///
/// If `configured_hosts` is set, the hostname will be considered resolvable if it is listed as an alias for any given IP address.
fn hostname_resolveable(
    hostname: &str,
    configured_hosts: Option<&bottlerocket_modeled_types::EtcHostsEntries>,
) -> bool {
    // If the hostname is in our configured hosts, then it *will* be resolvable when /etc/hosts is rendered.
    // Note that DNS search paths in /etc/resolv.conf are not relevant here, as they are not checked when searching /etc/hosts.
    if let Some(etc_hosts_entries) = configured_hosts {
        for (_, alias_list) in etc_hosts_entries.iter_merged() {
            if alias_list.iter().any(|alias| alias == hostname) {
                return true;
            }
        }
    }

    // Attempt to resolve the hostname
    match lookup_host(hostname) {
        Ok(ip_list) => {
            // If the list of IPs is empty or resolves to localhost, consider the hostname
            // unresolvable
            let resolves_to_localhost = ip_list
                .iter()
                .any(|ip| ip == &IPV4_LOCALHOST || ip == &IPV6_LOCALHOST);

            !(ip_list.is_empty() || resolves_to_localhost)
        }
        Err(e) => {
            trace!("DNS hostname lookup failed: {},", e);
            false
        }
    }
}

// =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=   =^..^=

#[cfg(test)]
mod test_join_node_taints {
    use super::*;
    use handlebars::RenderError;
    use serde::Serialize;
    use serde_json::json;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("join_node_taints", Box::new(join_node_taints));

        registry.render_template(tmpl, data)
    }

    #[test]
    fn basic() {
        let result = setup_and_render_template(
            "{{ join_node_taints map }}",
            &json!({"map":{"key1": ["value1:NoSchedule"], "key2": ["value2:NoSchedule"]}}),
        )
        .unwrap();
        assert_eq!(result, "key1=value1:NoSchedule,key2=value2:NoSchedule")
    }

    #[test]
    fn none() {
        let result = setup_and_render_template("{{ join_node_taints map }}", &json!({})).unwrap();
        assert_eq!(result, "")
    }

    #[test]
    fn empty_map() {
        let result =
            setup_and_render_template("{{ join_node_taints map }}", &json!({"map":{}})).unwrap();
        assert_eq!(result, "")
    }

    #[test]
    fn more_than_one() {
        let result = setup_and_render_template(
            "{{ join_node_taints map }}",
            &json!({"map":{"key1": ["value1:NoSchedule","value1:NoExecute"], "key2": ["value2:NoSchedule"]}}),
        )
        .unwrap();
        assert_eq!(
            result,
            "key1=value1:NoSchedule,key1=value1:NoExecute,key2=value2:NoSchedule"
        )
    }
}

#[cfg(test)]
mod test_ecr_registry {
    use super::*;
    use handlebars::RenderError;
    use serde::Serialize;
    use serde_json::json;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("ecr-prefix", Box::new(ecr_prefix));

        registry.render_template(tmpl, data)
    }

    const ECR_REGISTRY_TESTS: &[(&str, &str)] = &[
        (
            "eu-central-1",
            "328549459982.dkr.ecr.eu-central-1.amazonaws.com/bottlerocket-admin:v0.5.1",
        ),
        (
            "af-south-1",
            "917644944286.dkr.ecr.af-south-1.amazonaws.com/bottlerocket-admin:v0.5.1",
        ),
        // Test fallback url
        (
            "xy-ztown-1",
            "328549459982.dkr.ecr.us-east-1.amazonaws.com/bottlerocket-admin:v0.5.1",
        ),
        (
            "cn-north-1",
            "183470599484.dkr.ecr.cn-north-1.amazonaws.com.cn/bottlerocket-admin:v0.5.1",
        ),
        (
            "ap-south-2",
            "764716012617.dkr.ecr.ap-south-2.amazonaws.com/bottlerocket-admin:v0.5.1",
        ),
        (
            "ap-southeast-4",
            "731751899352.dkr.ecr.ap-southeast-4.amazonaws.com/bottlerocket-admin:v0.5.1",
        ),
        (
            "eu-central-2",
            "861738308508.dkr.ecr.eu-central-2.amazonaws.com/bottlerocket-admin:v0.5.1",
        ),
        (
            "eu-south-2",
            "620625777247.dkr.ecr.eu-south-2.amazonaws.com/bottlerocket-admin:v0.5.1",
        ),
        (
            "eu-isoe-west-1",
            "589460436674.dkr.ecr.eu-isoe-west-1.cloud.adc-e.uk/bottlerocket-admin:v0.5.1",
        ),
    ];

    const ADMIN_CONTAINER_TEMPLATE: &str =
        "{{ ecr-prefix settings.aws.region }}/bottlerocket-admin:v0.5.1";

    #[test]
    fn registry_urls() {
        for (region_name, expected_url) in ECR_REGISTRY_TESTS {
            let result = setup_and_render_template(
                ADMIN_CONTAINER_TEMPLATE,
                &json!({"settings": {"aws": {"region": *region_name}}}),
            )
            .unwrap();
            assert_eq!(result, *expected_url);
        }
    }
}

#[cfg(test)]
mod test_tuf_repository {
    use super::*;
    use handlebars::RenderError;
    use serde::Serialize;
    use serde_json::json;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut repository = Handlebars::new();
        repository.register_helper("tuf-prefix", Box::new(tuf_prefix));
        repository.register_helper("metadata-prefix", Box::new(metadata_prefix));

        repository.render_template(tmpl, data)
    }

    const METADATA_TEMPLATE: &str =
        "{{ tuf-prefix settings.aws.region }}{{ metadata-prefix settings.aws.region }}/2020-07-07/";

    const EXPECTED_URL_AF_SOUTH_1: &str = "https://updates.bottlerocket.aws/2020-07-07/";

    const EXPECTED_URL_XY_ZTOWN_1: &str = "https://updates.bottlerocket.aws/2020-07-07/";

    const EXPECTED_URL_CN_NORTH_1: &str =
        "https://bottlerocket-updates-cn-north-1.s3.dualstack.cn-north-1.amazonaws.com.cn/latest/metadata/2020-07-07/";

    const EXPECTED_URL_EU_ISOE_WEST_1: &str = "https://bottlerocket-updates-eu-isoe-west-1.s3.eu-isoe-west-1.cloud.adc-e.uk/latest/metadata/2020-07-07/";

    #[test]
    fn url_af_south_1() {
        let result = setup_and_render_template(
            METADATA_TEMPLATE,
            &json!({"settings": {"aws": {"region": "af-south-1"}}}),
        )
        .unwrap();
        assert_eq!(result, EXPECTED_URL_AF_SOUTH_1);
    }

    #[test]
    fn url_fallback() {
        let result = setup_and_render_template(
            METADATA_TEMPLATE,
            &json!({"settings": {"aws": {"region": "xy-ztown-1"}}}),
        )
        .unwrap();
        assert_eq!(result, EXPECTED_URL_XY_ZTOWN_1);
    }

    #[test]
    fn url_cn_north_1() {
        let result = setup_and_render_template(
            METADATA_TEMPLATE,
            &json!({"settings": {"aws": {"region": "cn-north-1"}}}),
        )
        .unwrap();
        assert_eq!(result, EXPECTED_URL_CN_NORTH_1);
    }

    #[test]
    fn url_eu_isoe_west_1() {
        let result = setup_and_render_template(
            METADATA_TEMPLATE,
            &json!({"settings": {"aws": {"region": "eu-isoe-west-1"}}}),
        )
        .unwrap();
        assert_eq!(result, EXPECTED_URL_EU_ISOE_WEST_1);
    }
}

#[cfg(test)]
mod test_host {
    use super::*;
    use handlebars::RenderError;
    use serde::Serialize;
    use serde_json::json;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("host", Box::new(host));

        registry.render_template(tmpl, data)
    }

    #[test]
    fn not_absolute_url() {
        assert!(setup_and_render_template(
            "{{ host url_setting }}",
            &json!({"url_setting": "example.com"}),
        )
        .is_err());
    }

    #[test]
    fn https() {
        let result = setup_and_render_template(
            "{{ host url_setting }}",
            &json!({"url_setting": "https://example.example.com/example/example"}),
        )
        .unwrap();
        assert_eq!(result, "example.example.com");
    }

    #[test]
    fn http() {
        let result = setup_and_render_template(
            "{{ host url_setting }}",
            &json!({"url_setting": "http://example.com"}),
        )
        .unwrap();
        assert_eq!(result, "example.com");
    }

    #[test]
    fn unknown_scheme() {
        let result = setup_and_render_template(
            "{{ host url_setting }}",
            &json!({"url_setting": "foo://example.com"}),
        )
        .unwrap();
        assert_eq!(result, "example.com");
    }
}

#[cfg(test)]
mod test_kube_reserve_memory {
    use super::*;
    use handlebars::RenderError;
    use serde::Serialize;
    use serde_json::json;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("kube_reserve_memory", Box::new(kube_reserve_memory));

        registry.render_template(tmpl, data)
    }

    const TEMPLATE: &str = r#""{{kube_reserve_memory  max-pods kube-reserved-memory}}""#;

    #[test]
    fn have_settings_1024_mi() {
        let result = setup_and_render_template(
            TEMPLATE,
            &json!({"max-pods": 29, "kube-reserved-memory": "1024Mi"}),
        )
        .unwrap();
        assert_eq!(result, "\"1024Mi\"");
    }

    #[test]
    fn no_settings_max_pods_0() {
        let result =
            setup_and_render_template(TEMPLATE, &json!({"max-pods": 0, "no-settings": "hi"}))
                .unwrap();
        assert_eq!(result, "\"255Mi\"");
    }

    #[test]
    fn no_settings_max_pods_29() {
        let result =
            setup_and_render_template(TEMPLATE, &json!({"max-pods": 29, "no-settings": "hi"}))
                .unwrap();
        assert_eq!(result, "\"574Mi\"");
    }

    #[test]
    fn max_pods_not_number() {
        setup_and_render_template(
            TEMPLATE,
            &json!({"settings": {"kubernetes": {"max-pods": "ten"}}}),
        )
        .unwrap_err();
    }
}

#[cfg(test)]
mod test_kube_reserve_cpu {
    use super::*;
    use handlebars::RenderError;
    use serde::Serialize;
    use serde_json::json;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("kube_reserve_cpu", Box::new(kube_reserve_cpu));

        registry.render_template(tmpl, data)
    }

    const TEMPLATE: &str = r#"{{kube_reserve_cpu settings.kubernetes.kube-reserved.cpu}}"#;

    #[test]
    fn kube_reserve_cpu_ok() {
        assert!(setup_and_render_template(TEMPLATE, &json!({"not-the-setting": "hi"})).is_ok());
    }

    #[test]
    fn kube_reserve_cpu_30_m() {
        let result = setup_and_render_template(
            TEMPLATE,
            &json!({"settings": {"kubernetes": {"kube-reserved": {"cpu": "30m"}}}}),
        )
        .unwrap();
        assert_eq!(result, "30m");
    }
}

#[cfg(test)]
mod test_kube_cpu_helper {
    use crate::helpers::kube_cpu_helper;
    use std::collections::HashMap;

    #[test]
    fn kube_cpu_helper_ok() {
        let mut cpu_reserved: HashMap<usize, &str> = HashMap::new();
        cpu_reserved.insert(0, "0m");
        cpu_reserved.insert(1, "60m");
        cpu_reserved.insert(2, "70m");
        cpu_reserved.insert(3, "75m");
        cpu_reserved.insert(4, "80m");
        cpu_reserved.insert(5, "82m");
        cpu_reserved.insert(6, "85m");
        cpu_reserved.insert(47, "187m");
        cpu_reserved.insert(48, "190m");

        for (num_cpus, expected_millicores) in cpu_reserved.into_iter() {
            assert_eq!(kube_cpu_helper(num_cpus).unwrap(), expected_millicores);
        }
    }
}

#[cfg(test)]
mod test_etc_hosts_helpers {
    use super::*;
    use handlebars::RenderError;
    use serde::Serialize;
    use serde_json::json;

    // A thin wrapper around the handlebars render_template method that includes
    // setup and registration of helpers
    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("localhost_aliases", Box::new(localhost_aliases));
        registry.register_helper("etc_hosts_entries", Box::new(etc_hosts_entries));

        registry.render_template(tmpl, data)
    }

    #[test]
    fn test_hostname_resolvable_respects_etc_hosts() {
        assert!(hostname_resolveable(
            "unresolveable.irrelevanthostname.tld",
            Some(
                &serde_json::from_str::<bottlerocket_modeled_types::EtcHostsEntries>(
                    r#"[["10.0.0.1", ["unresolveable.irrelevanthostname.tld"]]]"#
                )
                .unwrap()
            )
        ));
    }

    #[test]
    fn resolves_to_localhost_renders_entries() {
        // Given a configured hostname that does not resolve in DNS,
        // When /etc/hosts is rendered,
        // Then an additional alias shall be rendered pointing the configured hostname to localhost.
        let result = setup_and_render_template(
            r#"{{localhost_aliases "ipv4" hostname hosts}}"#,
            &json!({"hostname": "localhost"}),
        )
        .unwrap();
        assert_eq!(result, "localhost")
    }

    #[test]
    fn hostname_resolves_to_static_mapping() {
        // Given a configured hostname that does not resolve in DNS
        // and an /etc/hosts configuration that contains that hostname as an alias to an IP address,
        // When /etc/hosts is rendered,
        // Then an additional alias *shall not* be rendered pointing the hostname to localhost.
        let result = setup_and_render_template(
            r#"{{localhost_aliases "ipv4" hostname hosts}}"#,
            &json!({"hostname": "noresolve.bottlerocket.aws", "hosts": [["10.0.0.1", ["irrelevant", "noresolve.bottlerocket.aws"]]]}),
        )
        .unwrap();
        assert_eq!(result, "")
    }

    #[test]
    fn resolvable_hostname_renders_nothing() {
        let result = setup_and_render_template(
            r#"{{localhost_aliases "ipv6" hostname hosts}}"#,
            &json!({"hostname": "amazon.com", "hosts": []}),
        )
        .unwrap();
        assert_eq!(result, "")
    }

    #[test]
    fn static_localhost_mappings_render() {
        let result = setup_and_render_template(
            r#"127.0.0.1 localhost {{localhost_aliases "ipv4" hostname hosts}}"#,
            &json!({"hostname": "", "hosts": [["127.0.0.1", ["test.example.com", "test"]]]}),
        )
        .unwrap();
        assert_eq!(result, "127.0.0.1 localhost test.example.com test")
    }

    #[test]
    fn static_localhost_mappings_low_precedence() {
        let result = setup_and_render_template(
            r#"::1 localhost {{localhost_aliases "ipv6" hostname hosts}}"#,
            &json!({"hostname": "unresolvable.bottlerocket.aws", "hosts": [["::1", ["test.example.com", "test"]]]}),
        )
        .unwrap();
        assert_eq!(
            result,
            "::1 localhost unresolvable.bottlerocket.aws test.example.com test"
        )
    }

    #[test]
    fn hosts_unset_works() {
        let result = setup_and_render_template(
            r#"{{localhost_aliases "ipv4" hostname hosts}}"#,
            &json!({"hostname": "localhost"}),
        )
        .unwrap();
        assert_eq!(result, "localhost")
    }

    #[test]
    fn etc_hosts_entries_works() {
        let result = setup_and_render_template(
            r#"{{etc_hosts_entries hosts}}"#,
            &json!({"hosts": [["10.0.0.1", ["test.example.com", "test"]], ["10.0.0.2", ["test.example.com"]]]}),
        )
        .unwrap();
        assert_eq!(
            result,
            "10.0.0.1 test.example.com test\n10.0.0.2 test.example.com"
        )
    }

    #[test]
    fn etc_hosts_entries_ignores_localhost() {
        let result = setup_and_render_template(
            r#"{{etc_hosts_entries hosts}}"#,
            &json!({"hosts": [["10.0.0.1", ["test.example.com", "test"]], ["127.0.0.1", ["test.example.com"]], ["::1", ["test.example.com"]]]}),
        )
        .unwrap();
        assert_eq!(result, "10.0.0.1 test.example.com test")
    }

    #[test]
    fn etc_hosts_works_with_empty_hosts() {
        let result =
            setup_and_render_template(r#"{{etc_hosts_entries hosts}}"#, &json!({})).unwrap();
        assert_eq!(result, "")
    }
}

#[cfg(test)]
mod test_oci_spec {
    use super::{Containerd, Docker};
    use crate::helpers::*;
    use serde_json::json;
    use OciDefaultsResourceLimitType::*;

    // Custom struct that will always show that EFA is detected.
    struct EfaPresentDetector;
    impl EfaDetector for EfaPresentDetector {
        fn is_efa_attached(&self) -> Result<bool, TemplateHelperError> {
            Ok(true)
        }
    }

    // Custom struct that will always show that EFA is not detected.
    struct EfaNotPresentDetector;
    impl EfaDetector for EfaNotPresentDetector {
        fn is_efa_attached(&self) -> Result<bool, TemplateHelperError> {
            Ok(false)
        }
    }

    #[test]
    fn oci_spec_capabilities_test() {
        let json = json!({
            "kill": true,
            "lease": false,
            "mac-admin": true,
            "mknod": true
        });
        let capabilities = oci_spec_capabilities(&json).unwrap();
        let rendered = Containerd::get_capabilities(capabilities);
        assert_eq!(
            rendered,
            r#""bounding": ["CAP_KILL",
"CAP_MAC_ADMIN",
"CAP_MKNOD"],
"effective": ["CAP_KILL",
"CAP_MAC_ADMIN",
"CAP_MKNOD"],
"permitted": ["CAP_KILL",
"CAP_MAC_ADMIN",
"CAP_MKNOD"]
"#
        );
    }

    fn check_all_rlimits(
        (cap, bottlerocket, hard_limit, soft_limit): (OciDefaultsResourceLimitType, &str, i64, i64),
    ) {
        let json = json!({bottlerocket: {"hard-limit": hard_limit, "soft-limit": soft_limit}});
        let rlimits = oci_spec_resource_limits(&json).unwrap();
        let rendered = Containerd::get_resource_limits(&cap, rlimits.get(&cap).unwrap());
        let result = format!(
            r#"{{ "type": "{}", "hard": {}, "soft": {} }}"#,
            cap.to_linux_string(),
            hard_limit,
            soft_limit,
        );
        assert_eq!(rendered, result);
    }

    #[test]
    fn oci_spec_resource_limits_test() {
        let rlimits = [
            (MaxAddressSpace, "max-address-space", 2, 1),
            (MaxCoreFileSize, "max-core-file-size", 4, 3),
            (MaxCpuTime, "max-cpu-time", 5, 4),
            (MaxDataSize, "max-data-size", 6, 5),
            (MaxFileLocks, "max-file-locks", 7, 6),
            (MaxFileSize, "max-file-size", 8, 7),
            (MaxLockedMemory, "max-locked-memory", 9, 8),
            (MaxMsgqueueSize, "max-msgqueue-size", 10, 9),
            (MaxNicePriority, "max-nice-priority", 11, 10),
            (MaxOpenFiles, "max-open-files", 12, 11),
            (MaxPendingSignals, "max-pending-signals", 17, 15),
            (MaxProcesses, "max-processes", 13, 12),
            (MaxRealtimePriority, "max-realtime-priority", 18, 23),
            (MaxRealtimeTimeout, "max-realtime-timeout", 14, 13),
            (MaxResidentSet, "max-resident-set", 15, 14),
            (MaxStackSize, "max-stack-size", 16, 15),
        ];

        for rlimit in rlimits {
            check_all_rlimits(rlimit);
        }
    }

    #[test]
    fn generate_oci_resource_limits_efa_detected() {
        let json = json!({"max-open-files": {"hard-limit": 1, "soft-limit": 2}});
        let rlimits = generate_oci_resource_limits(&json, EfaPresentDetector {}).unwrap();
        let rendered = Containerd::get_resource_limits(
            &MaxLockedMemory,
            rlimits.get(&MaxLockedMemory).unwrap(),
        );
        assert_eq!(
            rendered,
            r#"{ "type": "RLIMIT_MEMLOCK", "hard": 18446744073709551615, "soft": 18446744073709551615 }"#
        );
    }

    #[test]
    fn generate_oci_resource_limits_efa_not_detected() {
        let json = json!({"max-open-files": {"hard-limit": 1, "soft-limit": 2}});
        let rlimits = generate_oci_resource_limits(&json, EfaNotPresentDetector {}).unwrap();
        // If EFA is not detected, we will not set the max-locked-memory rlimit
        assert_eq!(rlimits.get(&MaxLockedMemory), None)
    }

    #[test]
    fn oci_spec_max_locked_memory_as_unlimited_resource_limit_test() {
        let json = json!({"max-locked-memory": {"hard-limit": "unlimited", "soft-limit": 18}});
        let rlimits = oci_spec_resource_limits(&json).unwrap();
        let rendered = Containerd::get_resource_limits(
            &MaxLockedMemory,
            rlimits.get(&MaxLockedMemory).unwrap(),
        );

        assert_eq!(
            rendered,
            r#"{ "type": "RLIMIT_MEMLOCK", "hard": 18446744073709551615, "soft": 18 }"#
        );
    }

    #[test]
    fn oci_spec_max_locked_memory_as_minus_one_resource_limit_test() {
        let json = json!({"max-locked-memory": {"hard-limit": -1, "soft-limit": 18}});
        let rlimits = oci_spec_resource_limits(&json).unwrap();
        let rendered = Containerd::get_resource_limits(
            &MaxLockedMemory,
            rlimits.get(&MaxLockedMemory).unwrap(),
        );
        assert_eq!(
            rendered,
            r#"{ "type": "RLIMIT_MEMLOCK", "hard": 18446744073709551615, "soft": 18 }"#
        );
    }

    #[test]
    fn oci_spec_capabilities_docker_test() {
        let json = json!({
            "kill": true,
            "lease": false,
            "mac-admin": true,
            "mknod": true
        });
        let capabilities = oci_spec_capabilities(&json).unwrap();
        let rendered = Docker::get_capabilities(capabilities);
        assert_eq!(
            rendered,
            r#"["CAP_KILL",
"CAP_MAC_ADMIN",
"CAP_MKNOD"]"#
        );
    }

    #[test]
    fn oci_spec_resource_limits_test_docker() {
        let json = json!({"max-open-files": {"hard-limit": 1, "soft-limit": 2}});
        let rlimits = oci_spec_resource_limits(&json).unwrap();
        let rendered =
            Docker::get_resource_limits(&MaxOpenFiles, rlimits.get(&MaxOpenFiles).unwrap());
        assert_eq!(
            rendered,
            r#" "nofile":{ "Name": "nofile", "Hard": 1, "Soft": 2 }"#
        );
    }

    #[test]
    fn oci_spec_max_locked_memory_as_unlimited_docker_resource_limit_test() {
        let json = json!({"max-locked-memory": {"hard-limit": "unlimited", "soft-limit": 18}});
        let rlimits = oci_spec_resource_limits(&json).unwrap();
        let rendered =
            Docker::get_resource_limits(&MaxLockedMemory, rlimits.get(&MaxLockedMemory).unwrap());

        assert_eq!(
            rendered,
            r#" "memlock":{ "Name": "memlock", "Hard": -1, "Soft": 18 }"#
        );
    }
}

#[cfg(test)]
mod test_ecs_metadata_service_limits {
    use crate::helpers::ecs_metadata_service_limits;
    use handlebars::{Handlebars, RenderError};
    use serde::Serialize;
    use serde_json::json;

    const TEMPLATE: &str = r#"{{ecs_metadata_service_limits settings.rps settings.burst}}"#;

    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper(
            "ecs_metadata_service_limits",
            Box::new(ecs_metadata_service_limits),
        );

        registry.render_template(tmpl, data)
    }

    #[test]
    fn test_valid_ecs_metadata_service_limits() {
        let test_cases = [
            (json!({"settings": {"rps": 1, "burst": 1}}), r#"1,1"#),
            (json!({"settings": {"rps": 1}}), r#"1,60"#),
            (json!({"settings": {"burst": 1}}), r#"40,1"#),
            (json!({"settings": {}}), r#"40,60"#),
        ];

        test_cases.iter().for_each(|test_case| {
            let (config, expected) = test_case;
            let rendered = setup_and_render_template(TEMPLATE, config).unwrap();
            assert!(expected == &rendered);
        });
    }

    #[test]
    fn test_invalid_ecs_metadata_service_limits() {
        let test_cases = [
            json!({"settings": {"rps": [], "burst": 1}}),
            json!({"settings": {"rps": 1, "burst": []}}),
            json!({"settings": {"rps": [], "burst": []}}),
            json!({"settings": {"rps": {}, "burst": {}}}),
        ];

        test_cases.iter().for_each(|test_case| {
            let rendered = setup_and_render_template(TEMPLATE, test_case);
            assert!(rendered.is_err());
        });
    }
}

#[cfg(test)]
mod test_replace_ipv4_octet {
    use crate::helpers::replace_ipv4_octet;
    use handlebars::{Handlebars, RenderError};
    use serde::Serialize;
    use serde_json::json;

    const TEMPLATE: &str = r#"{{replace_ipv4_octet ip index value}}"#;

    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("replace_ipv4_octet", Box::new(replace_ipv4_octet));

        registry.render_template(tmpl, data)
    }

    #[test]
    fn test_valid_replace_ipv4_octet() {
        let test_cases = [
            (
                json!({"ip": "192.168.1.1", "index": 3, "value": "10"}),
                "192.168.1.10",
            ),
            (
                json!({"ip": "10.0.0.0", "index": 0, "value": "172"}),
                "172.0.0.0",
            ),
        ];

        test_cases.iter().for_each(|test_case| {
            let (config, expected) = test_case;
            let rendered = setup_and_render_template(TEMPLATE, config).unwrap();
            assert_eq!(expected, &rendered);
        });
    }

    #[test]
    fn test_invalid_replace_ipv4_octet() {
        let test_cases = [
            json!({"ip": "192.168.1.1", "index": 4, "value": "10"}), // Invalid index
            json!({"ip": "invalid-ip", "index": 3, "value": "10"}),  // Invalid IP
            json!({"ip": "192.168.1.1", "index": 3, "value": "257"}), // Invalid octet value
        ];

        test_cases.iter().for_each(|test_case| {
            let rendered = setup_and_render_template(TEMPLATE, test_case);
            assert!(rendered.is_err());
        });
    }
}

#[cfg(test)]
mod test_is_ipv4_is_ipv6 {
    use crate::helpers::{is_ipv4, is_ipv6};
    use handlebars::{Handlebars, RenderError};
    use serde::Serialize;
    use serde_json::json;

    const TEMPLATE_IPV4: &str = r#"{{is_ipv4 ipcidr}}"#;
    const TEMPLATE_IPV6: &str = r#"{{is_ipv6 ipcidr}}"#;

    fn setup_and_render_template<T>(
        tmpl: &str,
        data: &T,
        helper_name: &str,
    ) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        match helper_name {
            "is_ipv4" => registry.register_helper("is_ipv4", Box::new(is_ipv4)),
            "is_ipv6" => registry.register_helper("is_ipv6", Box::new(is_ipv6)),
            _ => panic!("Unknown helper"),
        }

        registry.render_template(tmpl, data)
    }

    #[test]
    fn test_valid_is_ipv4() {
        let test_cases = [
            (json!({"ipcidr": "192.168.1.0/24"}), "true"),
            (json!({"ipcidr": "2001:db8::/32"}), "false"),
        ];

        test_cases.iter().for_each(|test_case| {
            let (config, expected) = test_case;
            let rendered = setup_and_render_template(TEMPLATE_IPV4, config, "is_ipv4").unwrap();
            assert_eq!(expected, &rendered);
        });
    }

    #[test]
    fn test_valid_is_ipv6() {
        let test_cases = [
            (json!({"ipcidr": "2001:db8::/32"}), "true"),
            (json!({"ipcidr": "192.168.1.0/24"}), "false"),
        ];

        test_cases.iter().for_each(|test_case| {
            let (config, expected) = test_case;
            let rendered = setup_and_render_template(TEMPLATE_IPV6, config, "is_ipv6").unwrap();
            assert_eq!(expected, &rendered);
        });
    }

    #[test]
    fn test_invalid() {
        let test_cases = [json!({"ipcidr": "invalid-cidr"})];

        test_cases.iter().for_each(|test_case| {
            let rendered = setup_and_render_template(TEMPLATE_IPV4, test_case, "is_ipv4");
            assert!(rendered.is_err());
        });

        test_cases.iter().for_each(|test_case| {
            let rendered = setup_and_render_template(TEMPLATE_IPV6, test_case, "is_ipv6");
            assert!(rendered.is_err());
        });
    }
}

#[cfg(test)]
mod test_cidr_to_ipaddr {
    use crate::helpers::cidr_to_ipaddr;
    use handlebars::{Handlebars, RenderError};
    use serde::Serialize;
    use serde_json::json;

    const TEMPLATE: &str = r#"{{cidr_to_ipaddr ipcidr}}"#;

    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("cidr_to_ipaddr", Box::new(cidr_to_ipaddr));

        registry.render_template(tmpl, data)
    }

    #[test]
    fn test_valid_cidr_to_ipaddr() {
        let test_cases = [
            (json!({"ipcidr": "192.168.1.0/24"}), "192.168.1.0"),
            (json!({"ipcidr": "2001:db8::/32"}), "2001:db8::"),
        ];

        test_cases.iter().for_each(|test_case| {
            let (config, expected) = test_case;
            let rendered = setup_and_render_template(TEMPLATE, config).unwrap();
            assert_eq!(expected, &rendered);
        });
    }

    #[test]
    fn test_invalid_cidr_to_ipaddr() {
        let test_cases = [json!({"ipcidr": "invalid-cidr"})];

        test_cases.iter().for_each(|test_case| {
            let rendered = setup_and_render_template(TEMPLATE, test_case);
            assert!(rendered.is_err());
        });
    }
}

#[cfg(test)]
mod test_combined_template_for_ip_cidr {
    use crate::helpers::{cidr_to_ipaddr, is_ipv4, replace_ipv4_octet};
    use handlebars::{Handlebars, RenderError};
    use serde::Serialize;
    use serde_json::json;

    const TEMPLATE: &str = r#"
    {{#if (is_ipv4 ipcidr)}}
      {{ replace_ipv4_octet (cidr_to_ipaddr ipcidr) 3 "10" }}
    {{else}}
      {{cidr_to_ipaddr ipcidr}}a
    {{/if}}
    "#;

    fn setup_and_render_template<T>(tmpl: &str, data: &T) -> Result<String, RenderError>
    where
        T: Serialize,
    {
        let mut registry = Handlebars::new();
        registry.register_helper("is_ipv4", Box::new(is_ipv4));
        registry.register_helper("cidr_to_ipaddr", Box::new(cidr_to_ipaddr));
        registry.register_helper("replace_ipv4_octet", Box::new(replace_ipv4_octet));

        registry.render_template(tmpl, data)
    }

    #[test]
    fn test_combined_template_valid_cases() {
        let test_cases = [
            (json!({"ipcidr": "192.168.1.0/24"}), "192.168.1.10"), // IPv4 case with replacement
            (json!({"ipcidr": "2001:db8::/32"}), "2001:db8::a"),
        ];

        test_cases.iter().for_each(|test_case| {
            let (config, expected) = test_case;
            let rendered = setup_and_render_template(TEMPLATE, config)
                .unwrap()
                .trim()
                .to_string();
            assert_eq!(expected, &rendered);
        });
    }
}
