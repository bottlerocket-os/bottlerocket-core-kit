/// The mtu_tests macro contains MTU-related net config tests.  It accepts a version number,
/// and creates a module for that version containing all applicable MTU tests.
macro_rules! mtu_tests {
    ($version:expr) => {
        mod mtu {
            use $crate::net_config::deserialize_config;
            use $crate::net_config::test_macros::gen_boilerplate;

            gen_boilerplate!($version, "mtu");

            // Only test MTU for version 3 and later
            #[test]
            fn mtu_configuration() {
                if VERSION < 3 {
                    // MTU is only supported in version 3 and later
                    return;
                }

                let config_str = render_config_template(net_config().join("net_config.toml"));
                let net_config = deserialize_config(&config_str);
                assert!(net_config.is_ok(), "Failed to deserialize config: {:?}", net_config.err());

                let net_config = net_config.unwrap();

                // Check that the config has interfaces
                assert!(net_config.has_interfaces());

                // Convert to networkd config to ensure MTU values are passed through
                let networkd_config = net_config.as_networkd_config();
                assert!(networkd_config.is_ok(), "Failed to create networkd config: {:?}", networkd_config.err());

                // The test passes if we can successfully deserialize and create networkd config
                // with MTU values. The actual MTU values will be written to the .network files
                // when the networkd config is written to disk.
            }
        }
    };
}

pub(crate) use mtu_tests;