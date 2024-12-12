import express from "express";
import { Request, Response } from "express";

const app = express();

app.get("/", (req, res) => {
  res.send("Hello World!");
});

app.use(express.json());
app.set("trust proxy", true);
const serverData = [];
app.locals.serverData = serverData;

app.delete("/server/remove", async (req: Request, res: Response) => {
  const json = req.query;
  const id = json.id;
  const index = app.locals.serverData.findIndex((entry) => entry.id === id);
  if (index === -1) {
    res.status(404).send("Server not found");
    return;
  }
  app.locals.serverData.splice(index, 1);
  res.send("OK");
});

app.post("/server/heartbeat", async (req: Request, res: Response) => {
  const json = req.body;
  const ip = req.socket.remoteAddress;

  const ip_str = ip.replace(/^.*:/, "");

  // if the ip starts with 127.0
  if (ip_str.startsWith("172")) {
    console.log("IP is 172");
    console.log(req.headers["X-Real-IP"]);
    if (req.headers["X-Real-IP"]) {
      const realIp = req.headers["X-Real-IP"] as string;
      json.ip = realIp.replace(/^.*:/, "");
    }
  } else {
    json.ip = ip_str;
  }

  json.ip = ip_str;
  console.log(json);
  if (app.locals.serverData.some((entry) => entry.id === json.id)) {
    // make sure to return a 400 status code if the data is a duplicate, and update the data to be correct
    const duplicateIndex = app.locals.serverData.findIndex(
      (entry) => entry.id === json.id
    );
    app.locals.serverData[duplicateIndex] = json;
    res.status(200).send("Update data");
    return;
  }

  // if the ip & port is already in the list, then update the data
  const duplicateIndex = app.locals.serverData.findIndex(
    (entry) => entry.ip === json.ip && entry.port === json.port
  );

  if (duplicateIndex !== -1) {
    app.locals.serverData[duplicateIndex] = json;
    res.status(200).send("Update data");
    return;
  }

  app.locals.serverData.push(json);
  res.send("OK");
});

app.get("/server", async (req: Request, res: Response) => {
  console.log(app.locals.serverData);

  res.json(app.locals.serverData);
});

app.listen(3001, () => {
  console.log("Server started on port 3001");
});
