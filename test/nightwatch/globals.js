module.exports = {
	before : function(cb) {
		console.log("Requiring fetch polyfill");
		require('whatwg-fetch');
		cb();
	}
};
