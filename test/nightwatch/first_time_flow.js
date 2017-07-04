module.exports = {
  'Welcome page loads' : function (browser) {
    browser
      .url('http://localhost:5050')
      .waitForElementVisible('body', 1000);
    browser.end();
  }
};
