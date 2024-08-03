import express from "express";
const app = express();
const port = 3000;

let requestCount = 0;
app.use((req, res, next) => {
  requestCount++;
  console.log(
    "Time:",
    new Date(),
    "Request count: " + requestCount,
    req.method,
    req.url
  );
  next();
});
app.get("/", (req, res) => {
  if (Math.random() > 0.5) {
    return res.status(500).send("Internal Server Error");
  }
  res.send("Hello World!");
});

app.listen(port, () => {
  console.log(`Example app listening on port ${port}`);
});
