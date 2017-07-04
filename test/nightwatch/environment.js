var assert = require("assert");

module.exports = {
	'Fetch Web API' : function(browser) {
		assert.equal(typeof fetch, "function", "fetch function is not available");
	}
};
