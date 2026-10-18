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
  - For bugs, use the template provided.
  - Please provide as much information as you can - share your config, any logs, steps to reproduce...
  - If reporting an error, please ensure you use `IRONBAR_LOG` or `IRONBAR_FILE_LOG` set to `debug`.

## AI Policy

Code authored using AI *assistance* is permitted, although discouraged. 
This refers to using AI tools to help generate code you would have otherwise written yourself, 
or to help shape decisions that are led by yourself. Code generated entirely by AI will be handled on a case-by-case basis.
Fully "vibed" code (referring to letting an agent run end-to-end without user input) is not permitted, and will be rejected without further discussion.

If you do use AI for authoring code, please ensure the following:

- You should be at the helm, making architectural decisions that you are both fully aware of and fully understand.
- To back this up, discussions and descriptions on GitHub should be hand-written.
- Keep contributions smaller, and ensure they are agreed upon in an issue beforehand. This avoids burning tokens (and huge amounts of energy/water) on unwanted changes.

Contributions will be handled on a case-by-case basis, and if I feel the guidelines are not met,
the PR may be either edited to remove your authorship, or rejected entirely. 
A system is present on PRs that will automatically close the PR if enough AI smells are detected.
