export const ScriptApiBaseUrl = () => {
  if (process.env.NODE_ENV === "production") {
    return "http://192.168.1.31:2080";
  } else {
    return "http://localhost:2080";
  }
};

export const WebsocketBaseUrl = () => {
  if (process.env.NODE_ENV === "production") {
    return "ws://192.168.1.31:2080/ws";
  } else {
    return "ws://localhost:2080/ws";
  }
};
