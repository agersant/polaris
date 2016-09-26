[![Build Status](https://travis-ci.org/agersant/polaris.svg?branch=master)](https://travis-ci.org/agersant/polaris)

# Polaris

Polaris is a music streaming application, designed to let you enjoy your music collection from any computer or mobile device.

# Getting Started

## Requirements

- Windows 7 or newer

Linux support is on the radar but there is no ETA for this feature. 

## Installation

- Download the [latest installer](releases/latest)
- Run the installer
- That's it, you're done!

You can now start Polaris from the start menu or from your desktop, Polaris will also start automatically next time you restart your computer. You can tell when Polaris is running by its icon in the notification area (near the clock and volume controls).

## Basic Configuration

All configuration is done by editing the file located at `C:\Program Files\Polaris\Polaris\polaris.toml`. Note that Polaris needs to be restarted for configuration changes to be taken into account.

### Locating Your Music

Locate the following block in the configuration file:

```
[[mount_dirs]]
source = 'C:/Users/your_name/Music'		# Location of the directory on your computer
name = 'root'							# Public-facing name for this directory
```

Edit the source field so it points to your music. You can set the name field to anything you like, or keep the default value.

If you would like to stream music from other directories, you can add similar `[[mount_dirs]]` blocks after this one. The default configuration file comes with another such block, prefixed with `#` signs which indicate comments. Feel free to uncomment and use this block.

### Setting Up Users

Locate the following block in the configuration file:

```
[[users]]
name = 'your_first_user'
password = 'your_first_password'		# Passwords are stored unencrypted. Do not re-use a sensitive password!
```

Update the username and password to your liking. Heed the warning about password safety, do not re-use your email or banking password here! 

Similar to the `[[mount_dirs]]` block, you can add additional `[[users]]` block to create additional users.

### Test Run

Now would be a good time to try out your installation!

- Start Polaris using the shortcut on your desktop
- In your Web browser, access http://localhost:5050
- If all goes well, you will see a login form
- Enter your credentials as you typed them in the `[[users]]` block of your polaris.toml file
- Enjoy the music!

![Polaris Web UI](res/readme/web_ui.png?raw=true "Polaris Web UI")

### Streaming From Other Devices

If you're only interested in streaming on your local network, you can skip this section. If you want to stream from school, from work, or on the go, this is for you.

#### Dynamic DNS

You can access your Polaris installationfrom anywhere via your computer's public IP address, but there are two problems with that:
- IP addresses are difficult to remember
- Most ISP don't give you a fixed IP address

A solution to this problem is to set up Dynamic DNS, so that your installation can always be reached at a fixed URL.

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
6. Back to your Polaris configuration file, locate the following block:
```
# Use this section if you want Polaris to broadcast your IP to https://ydns.io 
# [ydns]
# host = 'your_hostname.ydns.eu'
# username = 'your_username'
# password = 'your_ydns_password'
```
- Uncomment this block by removing the # signs and leading space from all the lines
- Update the hostname to match what you set in step 5. (eg. yourdomain.ydns.eu)
- Update the username to the email address you use when creating your YDNS account
- You can find your YDNS API password on https://ydns.io. Click on the "User" icon in the top right and then `Preferences > API`.

#### Port Forwarding
Configure port forwarding on your router to redirect port 80 towards port 5050 on the computer where you run Polaris. The exact way to do this depends on your router manufacturer and model.

Don't forget to restart Polaris to apply your configuration changes, and access your music from other computers at http://yourdomain.ydns.eu
