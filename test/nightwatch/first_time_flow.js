module.exports = {
  'Welcome page loads' : function (browser) {
    browser
      .url('http://localhost:5050')
      .waitForElementVisible('#initial-setup-page', 1000);
	browser.expect.element("#initial-setup-page h2").text.to.contain("Welcome to Polaris");
    browser.end();
  }
};
