# Maintenance

## How to make a release

- `git pull` to not miss out on any recent contributions
- Update version number in Cargo.toml and commit the change (no need to push)
- Run `make_release.ps1`
- After CI completes, find the release on Github and write the changelog
- Move the release from Draft to Published

Note that the Github web UI will separate the release from the corresponding tag until published.

## How to change the database schema

- Add a new folder under `migrations` following the existing pattern
- Run `update_db_schema.bat`
