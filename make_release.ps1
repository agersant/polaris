Get-ChildItem "Cargo.toml" | ForEach-Object {
	$conf = $_ | Get-Content -raw
	$conf -match 'version\s+=\s+"(.*)"' | out-null
	$script:POLARIS_VERSION = $matches[1]
}

git tag -a $POLARIS_VERSION -m "Polaris $POLARIS_VERSION"
git push origin $POLARIS_VERSION
