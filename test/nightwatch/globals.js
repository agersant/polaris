module.exports = {
	before : function(cb) {
		console.log("Requiring fetch polyfill");
		fetch = require('node-fetch').fetch;
		cb();
	}
};
