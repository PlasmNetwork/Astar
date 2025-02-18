# Contributing

Welcome potential contributor of `Astar Network`! The Astar Network project (formerly known as Plasm Network) is a collection of **Open Source Projects** maintained by the Astar Team and Stake Technologies. We want to make contributing to this project as easy and transparent as possible.

This document will act as a starting point for those who want to be part of the Astar Ecosystem, which includes code contribution and community contribution.

## Types of Contribution

We are an open source project and will accept contributions which fix an issue, provide a new required feature, etc.
However, at this time, we're not accepting pull requests which aim to fix typos, change comment wording, rename variables, etc.

## Using GitHub

The Astar Network project uses GitHub as the main source control hosting service. Most forms of communication regarding changes to the code will be done within the issue board of the repository.

### Opening an Issue

Contributions within GitHub can take on the following forms:

- **Bug Report**: If you find any bugs or unexpected behaviors using the Astar node, please open an issue that describes the issue and other information that the developer may need when investigating.
- **User Questions**: Posting your question that is not addressed on our [official docs](https://docs.astar.network/), [Substrate docs](https://substrate.dev/docs/en/), or through other issue tickets helps us improve our wiki and keep the community informed. For any inquiries related to the usage/development of the code, please open an issue on our repository.
- **Feature Request**: If you have any suggestions or requests for a feature that can be made within a *relatively short development time*, feel free to open an issue that describes the 'what,' 'why,' 'how,' of the feature.

### Code Changes (Pull Request)

For those who wish to make changes to the source code, please ensure that there is an open issue that is related to the changes you're trying to make. *You must open an issue before you open a pull request* as this allows us to understand what changes will come and prevent stale pull requests. The issue should contain a rough description of how you are planning on changing the code to fix or add features. Once the repository maintainer gives the green light, you can fork the repository and open a pull request with your changes to our main branch (Dusty).

In short:

1. Open an issue regarding a bug fix or feature request (fill in our issue templates)
2. Briefly describe how you plan to make changes to the code
3. Fork the current default branch on _Astar_.
4. Open a pull request to the default branch (fill in our pull request template) and add the appropriate label.
5. Ensure all workflow checks have passed
6. Wait for the maintainers approval or change requests
7. Your code will be merged

Some notes: 
1. All the commits in your pull request are required to be signed and can be verified by GitHub. Refer to [this doc](https://docs.github.com/en/authentication/managing-commit-signature-verification/about-commit-signature-verification) for details.
2. You don't need to squash your commits manually, it will be done during merge.
3. In case of any merge conflicts please use merge not rebase.
4. In general: please avoid force pushes. Any existing reviews will be dropped after that.
5. There are e2e tests that run post merge. It's a good idea to run them in advance (especially for breaking changes). Refer to [this doc](https://github.com/AstarNetwork/e2e-tests) for details.

### Mandatory PR Labels
When submitting a Pull Request (PR), please make sure to include the necessary labels to help us categorize and prioritize your changes. PRs should contain at least one of the following mandatory labels:

- **runtime**: Use this label for changes or additions to the code that relate to runtime **(shiden, astar, shibuya, local)**
- **client**: PRs related to client.
- **ci**: PRs related to CI (workflows).
- **tests**: PRs related to xcm-simulator tests,rpc-tests or any other kind of test.
- **other**: PRs related to third-party, scripts and all other things which don't have a definite label.

### Coding Styles

Contributors should adhere to the [`rustfmt` styles](https://github.com/rust-lang/rustfmt).
Running `cargo fmt` prior to creating a commit will ensure everything is properly formatted.

### Branch Rules and Release Process

![branch-release](https://mermaid.ink/img/eyJjb2RlIjoiZ3JhcGggVERcbiAgICBGRUFUVVJFW2ZlYXR1cmUgYnJhbmNoXSAtLT58QWRkcyBuZXcgZmVhdHVyZXwgUFIocHVsbCByZXF1ZXN0KVxuICAgIEZJWFtmaXggYnJhbmNoXSAtLT58Rml4ZXMgYnVnfCBQUihwdWxsIHJlcXVlc3QpXG4gICAgRE9DW2RvY3VtZW50YXRpb24gYnJhbmNoXSAtLT58QWRkcyBkb2N1bWVudGF0aW9ufCBQUihvcGVuIHB1bGwgcmVxdWVzdClcbiAgICBQUiAtLT58SW5jcmVtZW50IHNwZWMgdmVyICYgTWVyZ2UgYnJhbmNofCBERVZbZGV2ZWxvcG1lbnQgYnJhbmNoXVxuICAgIERFViAtLT4gVEVTVChuZXR3b3JrIHRlc3RpbmcpXG4gICAgVEVTVCAtLT4gfEltcHJvdmVtZW50c3wgUFJcbiAgICBURVNUIC0tPiB8Q3JlYXRlIG5ldyByZWxlYXNlIHRhZ3wgUkVMRUFTRVtydW50aW1lIHVwZ3JhZGVdIiwibWVybWFpZCI6e30sInVwZGF0ZUVkaXRvciI6ZmFsc2UsImF1dG9TeW5jIjp0cnVlLCJ1cGRhdGVEaWFncmFtIjpmYWxzZX0)

All branch names should adhere to the following rules:

- `feature/*`: new features are added
- `doc/*`: documentation changes
- `fix/*`: bug fixes

Every major feature will be first deployed on our testnet parachain _Shibuya_ after which it can be deployed on one of the production networks.
The expected flow is:

`local testnet/dry-run` → `Shibuya` → `Shiden` → `Astar`

### Contributor Licenses

By contributing, you agree that your contributions will be licensed under the [GNU General Public License v3.0](https://github.com/AstarNetwork/Astar/blob/master/LICENSE) as is with the Astar source code. If you have any concerns regarding this matter, please contact the maintainer.

## Community Contribution

As a public blockchain network, Astar Network welcomes any contributions that help our community members and create the best blockchain network. Anyone can interact with the community through our [official forum](https://forum.astar.network/), [discord](https://discord.gg/astarnetwork), and [Telegram](https://t.me/PlasmOfficial). You can contribute to the community by actively participating in the forum discussions, helping other members, or sharing Astar Network with others.
