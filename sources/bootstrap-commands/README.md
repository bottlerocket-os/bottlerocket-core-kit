# bootstrap-commands

Current version: 0.1.0

## Bootstrap commands

`bootstrap-commands` ensures that bootstrap commands are executed as defined in the system
settings. It is called by `bootstrap-commands.service` which runs prior to the execution of
`bootstrap-containers`.

Each bootstrap command is a set of Bottlerocket API commands. The settings are first rendered
into a config file. Then, the system is configured by going through all the bootstrap commands
in lexicographical order and running all the commands inside it.

### Example:
Given a bootstrap command called `001-test-bootstrap-commands` with the following configuration:

```toml
[settings.bootstrap-commands.001-test-bootstrap-commands]
commands = [[ "apiclient", "set", "motd=helloworld"]]
essential = true
mode = "always"
```
This would set `/etc/motd` to "helloworld".

## Additional Information:
Certain valid `apiclient` commands that work in a session may fail in `bootstrap-commands`
due to relevant services not running at the time of the launch of the systemd service.

### Example:
```toml
[settings.bootstrap-commands.001-test-bootstrap-commands]
commands = [[ "apiclient", "exec", "admin", "ls"]]
essential = true
mode = "always"
```
This command fails because `bootstrap-commands.service` which calls this binary is launched
prior to `preconfigured.target` while `host-containers@.service` which is a requirement for
running "exec" commands are launched after preconfigured.target.

## Colophon

This text was generated using [cargo-readme](https://crates.io/crates/cargo-readme), and includes the rustdoc from `src/main.rs`.
