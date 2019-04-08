[![Linux Build Status](https://travis-ci.org/agersant/polaris.svg?branch=master)](https://travis-ci.org/agersant/polaris)
[![Windows Build Status](https://ci.appveyor.com/api/projects/status/w0gsnq7mo4bu0wne/branch/master?svg=true)](https://ci.appveyor.com/project/agersant/polaris)

<img src="res/readme/logo.png?raw=true"/>
Polaris is a music streaming application, designed to let you enjoy your music collection from any computer or mobile device. Polaris works by streaming your music directly from your own computer, without uploading it to a third-party. It is free and open-source software, without any kind of premium version. The only requirement is that your computer stays on while it streams music!

# Getting Started

## Requirements

One of the following:
- Windows 7 or newer
- Linux (any reasonably modern distribution should do)

## Installation

### Windows
1. Download the [latest installer](https://github.com/agersant/polaris/releases/latest) (you want the .msi file)
2. Run the installer
3. That's it, you're done!

You can now start Polaris from the start menu or from your desktop, Polaris will also start automatically next time you restart your computer. You can tell when Polaris is running by its icon in the notification area (near the clock and volume controls).

### Linux

#### Dependencies

1. Install OpenSSL and its headers. This is most likely available from your distribution's package manager. For instance on Ubuntu, execute `sudo apt-get install libssl-dev`
2. Install the nightly Rust compiler by executing `curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly` or using an [alternative method](https://www.rust-lang.org/en-US/install.html)

#### Polaris installation
1. Download the [latest release]((https://github.com/agersant/polaris/releases/latest)) of Polaris (you want the .tar.gz file)
2. Extract the polaris archive in a directory and open a terminal in that directory
3. Execute `make install` (this may take several minutes)

This installation process puts the polaris executable in `~/.local/bin/polaris` and several data files under `~/.local/share/polaris`.

From here, you might want to adjust your system to run Polaris on login using Cron, Systemd or whichever method your distribution endorses.

If you want to uninstall Polaris, execute `make uninstall` from the extracted archive's directory. This will simply delete the directories created by the install process.

### In a docker container

To run polaris from a Docker container, please follow instructions from the [docker-polaris](https://github.com/ogarcia/docker-polaris) repository.

## Test Run

- Start Polaris using the shortcut on your desktop (Windows) or by running the executable in `~/.local/bin/polaris` (Linux)
- In your Web browser, access http://localhost:5050
- You will see a welcome page that will guide you through the Polaris configuration

![Polaris Web UI](res/readme/web_ui.png?raw=true "Polaris Web UI")

## Streaming From Other Devices

If you're only interested in streaming on your local network, you can skip this section. If you want to stream from school, from work, or on the go, this is for you.

### Dynamic DNS

You can access your Polaris installation from anywhere via your computer's public IP address, but there are two problems with that:
- IP addresses are difficult to remember
- Most ISP don't give you a fixed IP address

A solution to these problems is to set up Dynamic DNS, so that your installation can always be reached at a fixed URL.

The steps below will walk you through setting up YDNS and Polaris to give your installation a fixed URL. If you have another solution in mind, or prefer using another Dynamic DNS service, skip to the next section.

1. Register for a free account on https://ydns.io
2. On the YDNS website, access the "My Hosts" page and press the + sign for "Add Host"
3. Fill the host form as described below:
	- Domain: ydns.eu
	- Name: This part is up to you, whatever you enter will be in the URL you use to access Polaris
	- Content: Leave the default. Take a note whether the value looks like a IPv4 address (format: xxx.xxx.xxx.xxx) or a IPv6 address (format: xxxx:xxxx:xxxx:xxxx:xxxx:xxxx:xxxx:xxxx)
	- Type: Dynamic IP
4. If the content field looked like a IPv4 address:	skip to step #6
5. If the content field looked like a IPv6 address:
	- Click on your host name (eg. yourdomain.ydns.eu)
    - You should now see a page which looks like this:
	![YDNS Records](res/readme/ydns_records.png?raw=true "YDNS Records")
	- Click on the green "+" icon on the right
	- Fill out the new form as described:
		- Make sure the `Type` field is set to `A`
		- Set content to 0.0.0.0
	- You should now be back on the "records" page which was pictured above
	- Click on the ID number on the left (#28717 in the example above) of the column that has AAAA listed as its "Type".
	- Click on the red trash can icon in the corner to delete this record
	- Done!
6. In the Polaris web interface, access the `Dynamic DNS` tab of the settings screen:
- Update the hostname field to match what you set in step 5. (eg. http://yourdomain.ydns.eu)
- Update the username field to the email address you use when creating your YDNS account
- Update the password field with your YDNS API password. You can find this password on https://ydns.io: click on the "User" icon in the top right and then `Preferences > API`.

### Port Forwarding
Configure port forwarding on your router to redirect port 80 towards port 5050 on the computer where you run Polaris. The exact way to do this depends on your router manufacturer and model.

Don't forget to restart Polaris to apply your configuration changes, and access your music from other computers at http://yourdomain.ydns.eu

## Additional clients
When you install Polaris, it comes with a web interface which can be accessed using any modern browser. However, it may be more convenient to use a native app on your mobile device. Currently, the only such app is the official [Polaris client for Android](https://github.com/agersant/polaris-android).

# Documentation

The Polaris server API is documented [here](https://agersant.github.io/polaris/swagger/). Please note that this Swagger page does not point to any live Polaris server so the `Try it out!` buttons are not expected to work.

Feel free to open Github issues (or Pull Requests) if clarifications are needed.
