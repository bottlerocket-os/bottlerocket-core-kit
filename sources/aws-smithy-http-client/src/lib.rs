/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

#![cfg_attr(docsrs, feature(doc_cfg))]

//! HTTP client implementation for smithy-rs generated code.
//!
//! # Crate Features
//!
//! - `default-client`: Enable default HTTP client implementation (based on hyper 1.x).
//! - `rustls-ring`: Enable TLS provider based on `rustls` using `ring` as the crypto provider
//! - `rustls-aws-lc`: Enable TLS provider based on `rustls` using `aws-lc` as the crypto provider
//! - `rustls-aws-lc-fips`: Same as `rustls-aws-lc` feature but using a FIPS compliant version of `aws-lc`

#![warn(
    missing_docs,
    rustdoc::missing_crate_level_docs,
    unreachable_pub,
    rust_2018_idioms
)]

/// Default HTTP and TLS connectors
#[cfg(feature = "default-client")]
pub(crate) mod client;
#[cfg(feature = "default-client")]
pub use client::{default_connector, proxy, tls, Builder, Connector, ConnectorBuilder};

mod error;
pub use error::HttpClientError;

#[allow(unused_macros, unused_imports)]
#[macro_use]
pub(crate) mod cfg {
    /// Any TLS provider enabled
    macro_rules! cfg_tls {
        ($($item:item)*) => {
            $(
                #[cfg(any(
                    feature = "rustls-aws-lc",
                    feature = "rustls-aws-lc-fips",
                    feature = "rustls-ring",
                ))]
                #[cfg_attr(docsrs, doc(cfg(any(
                    feature = "rustls-aws-lc",
                    feature = "rustls-aws-lc-fips",
                    feature = "rustls-ring",
                ))))]
                $item
            )*
        }
    }

    /// Any rustls provider enabled
    macro_rules! cfg_rustls {
        ($($item:item)*) => {
            $(
                #[cfg(any(
                    feature = "rustls-aws-lc",
                    feature = "rustls-aws-lc-fips",
                    feature = "rustls-ring"
                ))]
                #[cfg_attr(docsrs, doc(cfg(any(feature = "rustls-aws-lc", feature = "rustls-aws-lc-fips", feature = "rustls-ring"))))]
                $item
            )*
        }
    }

    pub(crate) use cfg_rustls;
    pub(crate) use cfg_tls;
}
