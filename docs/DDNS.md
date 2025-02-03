# Streaming from other devices

These instructions apply to users running Polaris on a home network. When deploying to cloud services or VPS, configurations requirements will differ.

## Port forwarding

Configure port forwarding on your router to redirect port 80 traffic towards port 5050 towards the computer running Polaris. The exact way to do this depends on your router manufacturer and model.

## Dynamic DNS

You can access your Polaris installation from anywhere via your computer's public IP address, but there are two problems with that:
- IP addresses are difficult to remember
- Most ISP don't give you a fixed IP address

A solution to these problems is to set up Dynamic DNS, so that your installation can always be reached at a fixed URL.

1. Reserve a URL with a dynamic DNS provider such as https://www.duckdns.org/ or https://freemyip.com/.
2. The dynamic DNS provider gives you a unique Update URL that can be used to tell them where to send traffic. For example, `freemyip.com` gives you this URL immediately after claiming a subdomain. Other providers may show it in your profile page, etc.
3. Access your Polaris instance (http://localhost:5050 by default).
4. Go to the `Setting page` and into the `Dynamic DNS` section.
5. Set the Update URL to the one you obtained in step 2.
