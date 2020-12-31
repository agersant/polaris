if (!(Test-Path env:POLARIS_VERSION)) {
  throw "POLARIS_VERSION environment variable is not defined"
}

""
"Compiling executable"
# TODO: Uncomment the following once Polaris can do variable expansion of %LOCALAPPDATA%
# And remove the code setting these as defaults in `service/mod.rs`
# $script:INSTALL_DIR = "%LOCALAPPDATA%\Permafrost\Polaris"
# $env:POLARIS_WEB_DIR = "$INSTALL_DIR\web"
# $env:POLARIS_SWAGGER_DIR = "$INSTALL_DIR\swagger"
# $env:POLARIS_DB_DIR = "$INSTALL_DIR"
# $env:POLARIS_LOG_DIR = "$INSTALL_DIR"
# $env:POLARIS_CACHE_DIR = "$INSTALL_DIR"
# $env:POLARIS_PID_DIR = "$INSTALL_DIR"
cargo rustc --release --features "ui" -- -o ".\target\release\polaris.exe"
cargo rustc --release -- -o ".\target\release\polaris-cli.exe"

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
$wxs.Wix.Product.SetAttribute("Version", $env:POLARIS_VERSION)
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
& $light_exe -dWebUIDir=".\release\tmp\web" -dSwaggerUIDir=".\release\tmp\swagger" -wx -ext WixUtilExtension -ext WixUIExtension -spdb -sw1076 -sice:ICE38 -sice:ICE64 -out .\release\polaris.msi .\release\tmp\installer.wixobj .\release\tmp\web_ui_fragment.wixobj  .\release\tmp\swagger_ui_fragment.wixobj

"Cleaning up"
Remove-Item -Recurse .\release\tmp

