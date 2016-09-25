"Compiling resource file"
RC /fo res\application.res res\application.rc

""
"Compiling executable"
cargo rustc --release --features "ui" -- -C link-args="/SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup res\application.res"

""
"Creating output directory"
New-Item .\release\windows -type directory -Force | Out-Null
Remove-Item -Recurse .\release\windows\*

""
"Copying to output directory"
Copy-Item .\target\release\polaris.exe .\release\windows\
Copy-Item .\res\libeay32.dll .\release\windows\
Copy-Item .\res\libeay32md.dll .\release\windows\
Copy-Item .\res\DefaultConfig.toml .\release\windows\polaris.toml
Copy-Item .\web\ .\release\windows\ -recurse

""
"Creating installer"
candle -wx -arch x64 -out .\release\windows\installer.wixobj .\res\installer.wxs
light  -wx -out .\release\windows\Polaris_0.1.0.msi .\release\windows\installer.wixobj

""
Read-Host -Prompt "All clear! Press Enter to exit"
