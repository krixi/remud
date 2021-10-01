import {
  BehaviorSubject,
  delay,
  distinctUntilChanged,
  filter,
  Observable,
  retryWhen,
  share,
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
  subject.pipe(retryWhen((errors) => errors.pipe(delay(5000))));
  // note: we deliberately don't subscribe here; we'll only subscribe when someone calls .on(...), which
  // makes it so that the components don't lose messages

  return {
    connectionStatus: connected.asObservable().pipe(distinctUntilChanged()),
    disconnect() {
      subject.complete();
      connected.complete();
    },
    on<T>(eventName: string): Observable<T> {
      return subject.asObservable().pipe(
        share(),
        filter((x) => x.type === eventName)
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
