import React, {
  FormEvent,
  ReactNode,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Subscription, timer } from "rxjs";
import Prism, { Token } from "prismjs";
import "prismjs/components/prism-rust";
import "prismjs/themes/prism-tomorrow.css";
import { useEditable } from "use-editable";
import { CompileError, Script, Trigger } from "../models/scripts-api";
import { useScriptsApi } from "../hooks/use-scripts-api";
import { ScriptApiBaseUrl } from "../env";
import { useHistory } from "react-router-dom";
import { CompileStatusCheckbox } from "./compile-status";
import "./script-form.css";

export interface ScriptFormProps {
  isCreate: boolean;
  script?: Script;
}

const compilerErrorToString = (err?: CompileError): string => {
  if (!err) {
    return "";
  }
  const { line, position, message } = err;
  return `${
    line && position
      ? `Compile failed: Line ${line}, Position ${position}: `
      : ``
  }${message}`;
};

export const ScriptForm: React.FC<ScriptFormProps> = ({ isCreate, script }) => {
  const history = useHistory();
  const { upsert, compile, remove } = useScriptsApi(ScriptApiBaseUrl());
  const [name, setName] = useState("");
  const [trigger, setTrigger] = useState<Trigger>(Trigger.Say);
  const [code, setCode] = useState("");
  const [err, setErr] = useState<CompileError | undefined>();
  const [create, setCreate] = useState(isCreate);
  const [saved, setSaved] = useState("");

  useEffect(() => {
    setName(script?.name || "");
    setCode(script?.code || "");
    setTrigger(script?.trigger || Trigger.Say);
    setErr(script?.error);
  }, [script]);

  const validateThenRun = async (e: FormEvent, fn: (e: FormEvent) => void) => {
    e.preventDefault();
    let errs = [];
    if (name === "") {
      errs.push("You need to specify a script name.");
    }
    if (code === "") {
      errs.push(
        "Code cannot be empty. Comment it out or delete the script if you don't want this code to run."
      );
    }
    setErr({ message: errs.join(" ") });
    if (errs.length > 0) {
      return;
    }
    await fn(e);
  };

  const submitForm = async (e: FormEvent) => {
    setSaved("");
    await upsert({ name: script?.name || name, trigger, code }, create)
      .then(() => {
        if (create) {
          history.push(`/scripts/${name}`);
        } else {
          setErr(undefined);
          setSaved("Saved!");
        }
      })
      .catch((reason: CompileError) => {
        const { isSaved } = reason;
        setSaved(isSaved ? "Saved with errors 🤷" : "");
        if (isSaved) {
          setCreate(false);
        }
        setErr(reason);
      });
  };

  const submitCompile = async (e: FormEvent) => {
    await compile({ name: script?.name || name, trigger, code })
      .then((value) => {
        console.log("got", value);
      })
      .catch((reason) => setErr(reason));
  };

  const submitDelete = async (e: FormEvent) => {
    await remove({ name: script?.name || name, trigger, code })
      .then(() => {
        // navigate back to the /scripts page if this was successful.
        history.push("/scripts");
      })
      .catch((reason) => setErr(reason));
  };

  useEffect(() => {
    let s: Subscription;
    if (saved) {
      s = timer(5000).subscribe({
        next: () => setSaved(""),
      });
    }
    return () => {
      if (s) {
        s.unsubscribe();
      }
    };
  }, [saved]);

  const savedCss = useMemo(() => (saved ? "fade-out" : "visible"), [saved]);

  return (
    <div className="w-full">
      <form onSubmit={(e) => validateThenRun(e, submitForm)}>
        {!create && (
          <div className="text-center mb-2">
            <div className="flex flex-row justify-center">
              <span className="mr-1">
                Editing <pre className="inline">{name}</pre>
              </span>
              <CompileStatusCheckbox error={err} />
            </div>
          </div>
        )}
        <div className="border border-gray-500 p-2 rounded">
          {create && <ScriptNameForm name={name} setName={setName} />}
          <ScriptTriggerForm trigger={trigger} setTrigger={setTrigger} />
          <ScriptCodeForm code={code} setCode={setCode} />
        </div>
        <div className="flex flex-row justify-between">
          <div className="text-center p-2">
            {err && (
              <div className="text-red-600">{compilerErrorToString(err)}</div>
            )}
          </div>

          <div className="mt-2 flex flex-row justify-end">
            <div className="mr-2">
              <input
                className="cursor-pointer bg-soft-gray btn"
                type="submit"
                value="Save"
              />
              <button
                className="btn"
                onClick={(e) => validateThenRun(e, submitCompile)}
              >
                Compile
              </button>
              {!create && (
                <button
                  className="btn"
                  onClick={(e) => validateThenRun(e, submitDelete)}
                >
                  Delete
                </button>
              )}
              <button className="btn" onClick={() => history.push("/scripts")}>
                Go Back
              </button>
            </div>
          </div>
        </div>
        <div className="flex flex-row-reverse">
          <span className={`${savedCss} p-2 success`}>{saved}</span>
        </div>
      </form>
    </div>
  );
};

interface ScriptNameFormProps {
  name: string;
  setName: (name: string) => void;
}

const ScriptNameForm: React.FC<ScriptNameFormProps> = ({ name, setName }) => {
  return (
    <div className="flex flex-row justify-between mb-2">
      <label className="w-1/6">Script name</label>
      <input
        className="p-0.5 rounded w-full bg-light-gray"
        type="text"
        value={name}
        onChange={(e) => setName(e.currentTarget.value)}
      />
    </div>
  );
};

interface ScriptTriggerFormProps {
  trigger: Trigger;
  setTrigger: (trigger: Trigger) => void;
}

const ScriptTriggerForm: React.FC<ScriptTriggerFormProps> = ({
  trigger,
  setTrigger,
}) => {
  return (
    <div className="flex flex-row justify-between mb-2">
      <label className="w-1/6">Trigger</label>
      <select
        className="w-full bg-light-gray"
        value={trigger}
        onChange={(e) => setTrigger(e.currentTarget.value as Trigger)}
      >
        {Object.values(Trigger).map((t) => (
          <option key={t} value={t}>
            {t}
          </option>
        ))}
      </select>
    </div>
  );
};

interface ScriptCodeFormProps {
  code: string;
  setCode: (trigger: string) => void;
}

const ScriptCodeForm: React.FC<ScriptCodeFormProps> = (props) => {
  return (
    <div className="flex flex-row justify-between">
      <label className="w-1/6">Code</label>
      <EditableCode lang="rust" {...props} />
    </div>
  );
};

// the following is inspired heavily by https://barhamon.com/post/Typescript_Nextjs_Prismjs
interface EditableCodeProps {
  lang: string;
  code: string;
  setCode: (code: string) => void;
}

const tokenToReact = (token: Token | string, i: number): ReactNode => {
  if (typeof token === "string") {
    return <span key={i}>{token}</span>;
  } else if (typeof token.content === "string") {
    return (
      <span key={i} className={`token ${token.type}`}>
        {token.content}
      </span>
    );
  } else if (Array.isArray(token.content)) {
    return (
      <span key={i} className={`token ${token.type}`}>
        {token.content.map(tokenToReact)}
      </span>
    );
  } else {
    return (
      <span key={i} className={`token ${token.type}`}>
        {tokenToReact(token.content, 0)}
      </span>
    );
  }
};

const EditableCode: React.FC<EditableCodeProps> = ({ lang, code, setCode }) => {
  const [tokens, setTokens] = useState<Array<string | Token>>([]);
  const editable = useRef(null);

  useEditable(editable, setCode);

  useEffect(() => {
    const tokens: Array<string | Token> = Prism.languages[lang]
      ? Prism.tokenize(code ? code : "\n", Prism.languages[lang])
      : [];
    setTokens(tokens);
  }, [code, lang]);

  return (
    <pre
      ref={editable}
      className={`language-${lang} p-0.5 rounded w-full h-96`}
      style={{ backgroundColor: "#505050" }}
    >
      {tokens.length && tokens.map(tokenToReact)}
    </pre>
  );
};
