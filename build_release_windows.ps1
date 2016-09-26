"Compiling resource file"
RC /fo res\application.res res\application.rc

""
"Compiling executable"
cargo rustc --release --features "ui" -- -C link-args="/SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup res\application.res"

""
"Creating output directory"
New-Item .\release\tmp -type directory -Force | Out-Null
Remove-Item -Recurse .\release\tmp\*

""
"Copying to output directory"
Copy-Item .\target\release\polaris.exe 	.\release\tmp\
Copy-Item .\res\libeay32.dll 			.\release\tmp\
Copy-Item .\res\libeay32md.dll 			.\release\tmp\
Copy-Item .\res\default_config.toml 	.\release\tmp\polaris.toml
Copy-Item .\web\ 						.\release\tmp\ -recurse
Copy-Item .\res\license.rtf				.\release\tmp\
Copy-Item .\res\banner.bmp				.\release\tmp\
Copy-Item .\res\dialog.bmp				.\release\tmp\

""
"Creating installer"
candle -wx -arch x64 					-out .\release\tmp\installer.wixobj 	.\res\installer.wxs
light  -wx -spdb -ext WixUIExtension	-out .\release\Polaris_0.1.0.msi 		.\release\tmp\installer.wixobj

"Cleaning up"
Remove-Item -Recurse .\release\tmp

""
Read-Host -Prompt "All clear! Press Enter to exit"
