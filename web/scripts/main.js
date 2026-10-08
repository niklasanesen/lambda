const apiUrl =
  location.hostname === "lambda.new" ? "https://api.lambda.new" : "";

for (const a of document.querySelectorAll("a[data-api]"))
  a.href = apiUrl + a.getAttribute("href");
