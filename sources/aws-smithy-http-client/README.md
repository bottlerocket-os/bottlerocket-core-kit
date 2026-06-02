# aws-smithy-http-client

HTTP client abstractions for generated smithy clients.

This is a Bottlerocket carry of selected components from the upstream
[aws-smithy-http-client](https://github.com/smithy-lang/smithy-rs) crate,
modified to support custom `CryptoProvider` injection for runtime FIPS
TLS configuration.

<!-- anchor_start:footer -->
This crate is part of the [AWS SDK for Rust](https://awslabs.github.io/aws-sdk-rust/) and the [smithy-rs](https://github.com/smithy-lang/smithy-rs) code generator. In most cases, it should not be used directly.
<!-- anchor_end:footer -->
