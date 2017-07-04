module.exports = {
	before : function(cb) {
		console.log("Requiring fetch polyfill");
		fetch = require('whatwg-fetch').fetch;
		cb();
	}
};
