const appUrl =
  location.hostname === "lambda.new"
    ? "https://app.lambda.new"
    : "http://127.0.0.1:8080";

document.querySelectorAll("a[data-app]").forEach((a) => {
  a.href = appUrl + a.getAttribute("href");
});
