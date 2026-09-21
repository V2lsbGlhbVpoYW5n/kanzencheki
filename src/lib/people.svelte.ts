import { sourceMessage } from "$lib/i18n.svelte";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import {
  desktop,
  librarySession,
  loadLibrary,
  catalogCommand,
} from "./session.svelte";
import {
  type Person,
  type PersonDraft,
  type PersonSpace,
  type PersonFile,
  type PersonDocument,
  normalize,
} from "./model";
import { startTask, watchTasks, updateTask, notify } from "./tasks.svelte";
const demoSpaces = new Map<string, PersonSpace>();
const demoBodies = new Map<string, string>();
export async function savePerson(draft: PersonDraft): Promise<Person> {
  if (desktop) {
    const p = await invoke<Person>("person_save", { draft });
    await loadLibrary(true);
    return p;
  }
  const name = draft.name.trim(),
    description = draft.description.trim();
  if (!name || name.length > 80 || description.length > 60)
    throw Error(sourceMessage("请填写名字和简短区分说明"));
  const old = librarySession.people.find((p) => p.id === draft.id);
  const same = librarySession.people.filter(
    (p) => p.id !== draft.id && normalize(p.name) === normalize(name),
  );
  if (same.length && (!description || (!old && !draft.allowDuplicate)))
    throw Error(
      sourceMessage("创建重名人物需要选择“创建同名人物”并填写区分说明"),
    );
  if (same.some((p) => normalize(p.description) === normalize(description)))
    throw Error(sourceMessage("同名人物的区分说明不能相同"));
  const p: Person = {
    id: old?.id ?? crypto.randomUUID(),
    name,
    description,
    aliases: [
      ...new Set([
        ...draft.aliases,
        ...(old && old.name !== name ? [old.name] : []),
      ]),
    ].filter((a) => normalize(a) !== normalize(name)),
    notes: draft.notes,
    deletedAt: null,
    createdAt: old?.createdAt ?? new Date().toISOString(),
  };
  librarySession.people = old
    ? librarySession.people.map((x) => (x.id === p.id ? p : x))
    : [...librarySession.people, p];
  for (const c of librarySession.photos)
    if (c.peopleIds?.includes(p.id))
      c.people = c.peopleIds.map(
        (id) => librarySession.people.find((x) => x.id === id)?.name ?? "",
      );
  return p;
}
export async function trashPerson(p: Person, restore = false, purge = false) {
  if (desktop)
    await catalogCommand(purge ? "person_purge" : "person_trash", {
      personId: p.id,
      restore,
    });
  else if (purge) {
    librarySession.people = librarySession.people.filter((x) => x.id !== p.id);
    for (const c of librarySession.photos) {
      c.peopleIds = c.peopleIds?.filter((id) => id !== p.id);
      c.people =
        c.peopleIds?.map(
          (id) => librarySession.people.find((x) => x.id === id)?.name ?? "",
        ) ?? [];
    }
    demoSpaces.delete(p.id);
  } else p.deletedAt = restore ? null : new Date().toISOString();
}
export async function getPersonSpace(personId: string): Promise<PersonSpace> {
  if (!desktop) {
    if (!demoSpaces.has(personId))
      demoSpaces.set(personId, { files: [], documents: [] });
    return demoSpaces.get(personId)!;
  }
  const s = await invoke<PersonSpace>("person_space", { personId });
  s.files = s.files.map((f) => ({ ...f, src: convertFileSrc(f.src) }));
  return s;
}
export async function importPersonFiles(personId: string) {
  await watchTasks();
  const taskId = startTask(sourceMessage("导入人物附件"));
  librarySession.busy = true;
  try {
    await invoke("person_files_import", { personId, taskId });
  } catch (e) {
    updateTask({
      id: taskId,
      title: sourceMessage("导入人物附件"),
      detail: String(e),
      state: "error",
      done: 0,
      total: 0,
    });
    throw e;
  } finally {
    librarySession.busy = false;
  }
  return getPersonSpace(personId);
}
export async function demoImport(personId: string, files: FileList) {
  const space = await getPersonSpace(personId);
  for (const file of files)
    space.files.unshift({
      id: crypto.randomUUID(),
      personId,
      filename: file.name,
      originalFilename: file.name,
      src: URL.createObjectURL(file),
      mimeType: file.type,
      byteSize: file.size,
      createdAt: new Date().toISOString(),
      available: true,
    });
  return { ...space };
}
export async function deletePersonFile(f: PersonFile) {
  if (desktop)
    await invoke("person_file_delete", { personId: f.personId, fileId: f.id });
  else {
    const s = await getPersonSpace(f.personId);
    s.files = s.files.filter((x) => x.id !== f.id);
  }
  return getPersonSpace(f.personId);
}
export async function readDocument(
  d: PersonDocument,
): Promise<{ body: string; revision: string }> {
  if (desktop)
    return invoke("person_document_read", {
      personId: d.personId,
      documentId: d.id,
    });
  const body = demoBodies.get(d.id) ?? "";
  return { body, revision: body };
}
export async function saveDocument(draft: {
  id?: string;
  personId: string;
  title: string;
  body: string;
  revision: string | null;
}) {
  if (desktop) await invoke("person_document_save", { draft });
  else {
    const s = await getPersonSpace(draft.personId);
    const id = draft.id ?? crypto.randomUUID();
    if (draft.id && draft.revision !== demoBodies.get(id))
      throw Error(sourceMessage("文章已被修改，请重新打开"));
    demoBodies.set(id, draft.body);
    s.documents = [
      {
        id,
        personId: draft.personId,
        title: draft.title,
        filename: id + ".md",
        updatedAt: new Date().toISOString(),
      },
      ...s.documents.filter((d) => d.id !== id),
    ];
  }
  return getPersonSpace(draft.personId);
}
export async function deleteDocument(d: PersonDocument) {
  if (desktop)
    await invoke("person_document_delete", {
      personId: d.personId,
      documentId: d.id,
    });
  else {
    const s = await getPersonSpace(d.personId);
    s.documents = s.documents.filter((x) => x.id !== d.id);
    demoBodies.delete(d.id);
  }
  return getPersonSpace(d.personId);
}

export async function renamePersonFile(file: PersonFile, name: string) {
  if (desktop)
    await invoke("person_file_rename", {
      personId: file.personId,
      fileId: file.id,
      name,
    });
  else {
    const s = await getPersonSpace(file.personId);
    if (!name.trim() || /[\\/:*?"<>|]/.test(name))
      throw Error(sourceMessage("请输入有效文件名"));
    if (s.files.some((f) => f.id !== file.id && f.filename === name))
      throw Error(sourceMessage("文件名已存在"));
    const f = s.files.find((f) => f.id === file.id)!;
    f.filename = name.trim();
  }
  return getPersonSpace(file.personId);
}
