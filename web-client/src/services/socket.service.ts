import {
  BehaviorSubject,
  delay,
  distinctUntilChanged,
  Observable,
  retryWhen,
  tap,
} from "rxjs";
import {
  webSocket,
  WebSocketSubject,
  WebSocketSubjectConfig,
} from "rxjs/webSocket";

export interface SocketService {
  connectionStatus: Observable<boolean>;
  disconnect(): void;
  on<T>(eventName: string): Observable<T>;
  emit<T>(eventName: string, data: T): void;
}

export const NewSocketService = (uri: string): SocketService => {
  console.log("initializing socket service");
  // Keep track of connection status as an observable.
  let connected = new BehaviorSubject(false);
  const conf: WebSocketSubjectConfig<any> = {
    url: uri,
    closeObserver: {
      next: (data) => connected.next(false),
    },
    openObserver: {
      next: (data) => connected.next(true),
    },
  };

  // create the connection

  const subject: WebSocketSubject<any> = webSocket(conf);
  subject
    .pipe(
      retryWhen((errors) =>
        errors.pipe(
          tap((err) => console.log("error connecting: ", err)),
          delay(5000)
        )
      )
    )
    // subscribe required for connection to be established.
    .subscribe({
      error: (err) => console.log(err),
    });

  return {
    connectionStatus: connected.asObservable().pipe(distinctUntilChanged()),
    disconnect() {
      subject.complete();
      connected.complete();
    },
    on<T>(eventName: string): Observable<T> {
      return subject.multiplex(
        () => ({ type: "sub", data: { topic: eventName } }),
        () => ({ type: "unsub", data: { topic: eventName } }),
        (message) => message.type === "output"
      );
    },
    emit<T>(eventName: string, data: T): void {
      subject.next({
        type: eventName,
        data: data,
      });
    },
  };
};
