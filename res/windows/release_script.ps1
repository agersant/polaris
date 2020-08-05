Get-ChildItem "Cargo.toml" | ForEach-Object {
  $conf = $_ | Get-Content -raw
  $conf -match 'version\s+=\s+"(.*)"' | out-null
  $script:POLARIS_VERSION = $matches[1]
}

"Compiling resource file"
$rc_exe = Join-Path "C:\Program Files (x86)\Windows Kits\10\bin\10.0.18362.0\x64" RC.exe
& $rc_exe /fo res\windows\application\application.res res\windows\application\application.rc

""
"Compiling executable"
cargo rustc --release --features "ui" -- -C link-args="/SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup res\windows\application\application.res"
cargo rustc --release -- -o ".\target\release\polaris-cli.exe" -C link-args="res\windows\application\application.res"

""
"Creating output directory"
New-Item .\release\tmp -type directory -Force | Out-Null
Remove-Item -Recurse .\release\tmp\*

""
"Copying to output directory"
Copy-Item .\res\windows\installer\license.rtf	.\release\tmp\
Copy-Item .\res\windows\installer\banner.bmp	.\release\tmp\
Copy-Item .\res\windows\installer\dialog.bmp	.\release\tmp\
Copy-Item .\target\release\polaris.exe 			  .\release\tmp\
Copy-Item .\target\release\polaris-cli.exe 		.\release\tmp\
Copy-Item .\web   								            .\release\tmp\web     -recurse
Copy-Item .\docs\swagger  					          .\release\tmp\swagger -recurse

""
"Inserting version number in installer config"
[xml]$wxs = Get-Content .\res\windows\installer\installer.wxs
$wxs.Wix.Product.SetAttribute("Version", $POLARIS_VERSION)
$wxs.Save('.\res\windows\installer\installer.wxs')

""
"Creating installer"
$heat_exe = Join-Path $env:WIX bin\heat.exe
& $heat_exe dir .\release\tmp\web\ -ag -g1 -dr AppDataPolaris -cg WebUI -sfrag -var wix.WebUIDir -out .\release\tmp\web_ui_fragment.wxs
& $heat_exe dir .\release\tmp\swagger\ -ag -g1 -dr AppDataPolaris -cg SwaggerUI -sfrag -var wix.SwaggerUIDir -out .\release\tmp\swagger_ui_fragment.wxs

$candle_exe = Join-Path $env:WIX bin\candle.exe
& $candle_exe -wx -ext WixUtilExtension -arch x64 -out .\release\tmp\web_ui_fragment.wixobj .\release\tmp\web_ui_fragment.wxs
& $candle_exe -wx -ext WixUtilExtension -arch x64 -out .\release\tmp\swagger_ui_fragment.wixobj .\release\tmp\swagger_ui_fragment.wxs
& $candle_exe -wx -ext WixUtilExtension -arch x64 -out .\release\tmp\installer.wixobj .\res\windows\installer\installer.wxs

$light_exe = Join-Path $env:WIX bin\light.exe
& $light_exe -dWebUIDir=".\release\tmp\web" -dSwaggerUIDir=".\release\tmp\swagger" -wx -ext WixUtilExtension -ext WixUIExtension -spdb -sw1076 -sice:ICE38 -sice:ICE64 -out .\release\Polaris_$POLARIS_VERSION.msi .\release\tmp\installer.wixobj .\release\tmp\web_ui_fragment.wixobj  .\release\tmp\swagger_ui_fragment.wixobj

"Cleaning up"
Remove-Item -Recurse .\release\tmp

