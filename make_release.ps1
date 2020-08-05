Get-ChildItem "Cargo.toml" | ForEach-Object {
	$conf = $_ | Get-Content -raw
	$conf -match 'version\s+=\s+"(.*)"' | out-null
	$script:POLARIS_VERSION = $matches[1]
}

git tag -d $POLARIS_VERSION
git push --delete origin $POLARIS_VERSION
git tag $POLARIS_VERSION
git push --tags
