# Streaming From Other Devices

If you're only interested in streaming on your local network, you can skip this section. If you want to stream from school, from work, or on the go, this is for you.

## Dynamic DNS

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
	![YDNS Records](res/ydns_records.png?raw=true "YDNS Records")
	- Click on the green "+" icon on the right
	- Fill out the new form as described:
		- Make sure the `Type` field is set to `A`
		- Set content to 0.0.0.0
	- You should now be back on the "records" page which was pictured above
	- Click on the ID number on the left for the row that has its `Type` listed as `AAAA` (#28717 in the picture above).
	- Click on the red trash can icon in the corner to delete this record
	- Done!
6. In the Polaris web interface, access the `Dynamic DNS` tab of the settings screen:
- Update the hostname field to match what you set in step 5. (eg. http://yourdomain.ydns.eu)
- Update the username field to the email address you use when creating your YDNS account
- Update the password field with your YDNS API password. You can find this password on https://ydns.io: click on the "User" icon in the top right and then `Preferences > API`.

## Port Forwarding
Configure port forwarding on your router to redirect port 80 towards port 5050 on the computer where you run Polaris. The exact way to do this depends on your router manufacturer and model.

Don't forget to restart Polaris to apply your configuration changes, and access your music from other computers at http://yourdomain.ydns.eu
