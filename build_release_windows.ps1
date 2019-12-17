Get-ChildItem "Cargo.toml" | ForEach-Object {
  $conf = $_ | Get-Content -raw
  $conf -match 'version\s+=\s+"(.*)"' | out-null
  $script:POLARIS_VERSION = $matches[1]
}

"Compiling resource file"
RC /fo res\windows\application\application.res res\windows\application\application.rc

""
"Compiling executable"
cargo rustc --release --features "ui" -- -C link-args="/SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup res\windows\application\application.res"

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
Copy-Item .\web\img								            .\release\tmp\web\img   -recurse
Copy-Item .\web\js								            .\release\tmp\web\js    -recurse
Copy-Item .\web\lib								            .\release\tmp\web\lib   -recurse
Copy-Item .\web\style								          .\release\tmp\web\style -recurse
Copy-Item .\web\tags								          .\release\tmp\web\tags  -recurse
Copy-Item .\web\favicon.png					          .\release\tmp\web\
Copy-Item .\web\index.html					          .\release\tmp\web\
Copy-Item .\docs\swagger  					          .\release\tmp\swagger   -recurse

""
"Creating installer"
heat dir .\release\tmp\web\ -ag -g1 -dr AppDataPolaris -cg WebUI -sfrag -var wix.WebUIDir -out .\release\tmp\web_ui_fragment.wxs
heat dir .\release\tmp\swagger\ -ag -g1 -dr AppDataPolaris -cg SwaggerUI -sfrag -var wix.SwaggerUIDir -out .\release\tmp\swagger_ui_fragment.wxs

candle -wx -ext WixUtilExtension -arch x64 -out .\release\tmp\web_ui_fragment.wixobj .\release\tmp\web_ui_fragment.wxs
candle -wx -ext WixUtilExtension -arch x64 -out .\release\tmp\swagger_ui_fragment.wixobj .\release\tmp\swagger_ui_fragment.wxs
candle -wx -ext WixUtilExtension -arch x64 -out .\release\tmp\installer.wixobj .\res\windows\installer\installer.wxs

light -dWebUIDir=".\release\tmp\web" -dSwaggerUIDir=".\release\tmp\swagger" -wx -ext WixUtilExtension -ext WixUIExtension -spdb -sw1076 -sice:ICE38 -sice:ICE64 -out .\release\Polaris_$POLARIS_VERSION.msi .\release\tmp\installer.wixobj .\release\tmp\web_ui_fragment.wixobj  .\release\tmp\swagger_ui_fragment.wixobj

"Cleaning up"
Remove-Item -Recurse .\release\tmp

""
Read-Host -Prompt "All clear! Press Enter to exit"
