I welcome contributions of any kind with open arms. That said, please do stick to some basics:

- For code contributions:
  - Ensure your code builds when using `--no-default-features`.
  - Fix any `cargo clippy` warnings, using at least the default feature set.
    - Where features are disabled, some unused code warnings are allowed.
  - Make sure your code is formatted using `cargo fmt`.
  - Keep any documentation up to date.
  - Please use [conventional commit](https://www.conventionalcommits.org/en/v1.0.0/) messages.
    This ensures your contributions are automatically included in the changelog.


- For PRs:
  - Please open an issue or discussion beforehand. 
    I'll accept most contributions, but it's best to make sure you're not working on something that won't get accepted :)


- For issues:
  - Please provide as much information as you can - share your config, any logs, steps to reproduce...
  - If reporting an error, please ensure you use `IRONBAR_LOG` or `IRONBAR_FILE_LOG` set to `debug`.
