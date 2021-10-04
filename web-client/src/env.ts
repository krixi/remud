export const ScriptApiBaseUrl = () => {
  if (process.env.NODE_ENV === "production") {
    return "https://uplink.city-six.com";
  } else {
    return "http://localhost:2080";
  }
};

export const WebsocketBaseUrl = () => {
  if (process.env.NODE_ENV === "production") {
    return "ws://uplink.city-six.com/ws";
  } else {
    return "ws://localhost:2080/ws";
  }
};
