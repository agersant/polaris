"Compiling resource file"
RC /fo res\application.res res\application.rc

"Compiling executable"
cargo rustc --release --features "ui" -- -C link-args="/SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup res\application.res"

"Creating output directory"
New-Item .\release\windows -type directory -Force
Remove-Item -Recurse .\release\windows\*

"Copying to output directory"
Copy-Item .\target\release\polaris.exe .\release\windows\
Copy-Item .\res\libeay32.dll .\release\windows\
Copy-Item .\res\libeay32md.dll .\release\windows\
Copy-Item .\Polaris.toml .\release\windows\
Copy-Item .\web\ .\release\windows\ -recurse

Read-Host -Prompt "All clear! Press Enter to exit"
