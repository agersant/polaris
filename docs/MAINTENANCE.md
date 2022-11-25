# Maintenance

## How to make a release

- Update CHANGELOG.md to reflect new release
- On Github, go to **Actions**, select the **Make Release** workflow and click **Run workflow**
- Select the branch to deploy (usually `master`)
- Input a user-facing version name (eg: **0.13.0**)
- Click the **Run workflow** button
- After CI completes, move the release from Draft to Published

## How to change the database schema

- Add a new folder under `migrations` following the existing pattern
- Run `update_db_schema.bat`
