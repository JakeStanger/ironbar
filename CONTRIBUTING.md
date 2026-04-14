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

## AI Policy

Code generated using AI assistance is permitted, although discouraged. 
Fully "vibed" code is not permitted.
All contributions will be scrutinised during review to the same level regardless of origin,
so you are expected to be able to understand and amend your code as required.

My general preference would be that you avoid LLMs, 
as they have a remarkably negative impact in just about every part of the world.
I personally do not use AI for authoring code.
