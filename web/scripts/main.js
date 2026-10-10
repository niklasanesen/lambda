const apiUrl =
  location.hostname === "lambda.new" ? "https://api.lambda.new" : "";

for (const a of document.querySelectorAll("a[data-api]"))
  a.href = apiUrl + a.getAttribute("href");

document.body.addEventListener("htmx:config:request", function (e) {
  e.detail.ctx.request.action = apiUrl + e.detail.ctx.request.action;
  e.detail.ctx.request.credentials = "include";
});
