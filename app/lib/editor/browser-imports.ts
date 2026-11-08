import { editorText } from './i18n';

// Explicit extensions also cover OS MIME registries that classify TS as source code.
export const EDITOR_MEDIA_ACCEPT={
  video:'video/*,.mp4,.m4v,.mov,.webm,.mkv,.avi,.flv,.ts,.mts,.m2ts,.mpg,.mpeg,.vob,.ogv',
  audio:'audio/*,.mp3,.wav,.flac,.ogg,.oga,.opus,.aac,.ac3,.aif,.aiff,.m4a',
  image:'image/png,image/jpeg,image/gif,image/webp,image/bmp,image/tiff,image/avif,image/svg+xml,.png,.jpg,.jpeg,.gif,.webp,.bmp,.tif,.tiff,.avif,.svg',
} as const;
export const EDITOR_FILE_ACCEPT = `${Object.values(EDITOR_MEDIA_ACCEPT).join(',')},.html,.htm,.srt,.vtt,.ttf,.otf,.woff,.woff2`;
export interface BrowserFileSelection { files: File[]; problems: string[] }
const MAX_FILES = 100;
const limitMessage = () => editorText('Import up to 100 files at a time. The remaining files were not added.');
const visible = (path: string) => !path.split('/').some(part => part.startsWith('.'));

/** Open synchronously from the user gesture; each consumer applies its own import limits. */
export function selectBrowserFiles({directory=false,accept='',multiple=true}:{directory?:boolean;accept?:string;multiple?:boolean}={}):Promise<File[]> {
  return new Promise((resolve,reject) => {
    const input = document.createElement('input');
    input.type = 'file'; input.multiple = multiple; input.accept = accept;
    input.webkitdirectory = directory; input.hidden = true;
    let settled=false;
    const cleanup=()=>{input.removeEventListener('change',changed);input.removeEventListener('cancel',cancelled);input.remove();};
    const finish = (files: File[]) => {
      if(settled)return;
      settled=true;cleanup();
      resolve(files.filter(file => !directory || visible(file.webkitRelativePath || file.name)));
    };
    const changed=()=>finish(Array.from(input.files??[])),cancelled=()=>finish([]);
    input.addEventListener('change', changed);
    input.addEventListener('cancel', cancelled);
    document.body.appendChild(input);
    try{input.click();}catch(error){settled=true;cleanup();reject(error);}
  });
}

/** Only the browser's explicit file selection is readable; no server or local path browsing. */
export async function chooseEditorFiles(directory = false, accept = EDITOR_FILE_ACCEPT, multiple = true): Promise<BrowserFileSelection> {
  const chosen=await selectBrowserFiles({directory,accept,multiple});
  return { files: chosen.slice(0, MAX_FILES), problems: chosen.length > MAX_FILES ? [limitMessage()] : [] };
}

/** Capture entries during drop/paste: browsers clear DataTransfer after the event returns. */
export function readDroppedFiles(transfer: DataTransfer): Promise<BrowserFileSelection> {
  const fallback = Array.from(transfer.files);
  const entries = Array.from(transfer.items).filter(item => item.kind === 'file').map(item => ({
    entry: item.webkitGetAsEntry?.(), file: item.getAsFile(),
  }));
  if (!entries.some(item => item.entry?.isDirectory)) return Promise.resolve({
    files: fallback.slice(0, MAX_FILES), problems: fallback.length > MAX_FILES ? [limitMessage()] : [],
  });
  return (async () => {
    const files: File[] = [], problems: string[] = [];
    let visited = 0, stopped = false;
    const visit = async (entry: FileSystemEntry, depth: number): Promise<void> => {
      if (stopped || !visible(entry.fullPath)) return;
      if (++visited > 4096 || depth > 16 || files.length >= MAX_FILES) {
        stopped = true; problems.push(editorText('The folder is too large or deeply nested. Import smaller groups of files.')); return;
      }
      try {
        if (entry.isFile) {
          files.push(await new Promise<File>((resolve, reject) => (entry as FileSystemFileEntry).file(resolve, reject)));
        } else if (entry.isDirectory) {
          const reader = (entry as FileSystemDirectoryEntry).createReader();
          while (!stopped) {
            const batch = await new Promise<FileSystemEntry[]>((resolve, reject) => reader.readEntries(resolve, reject));
            if (!batch.length) break;
            for (const child of batch) await visit(child, depth + 1);
          }
        }
      } catch {
        problems.push(editorText("Couldn't read {0}.", { 0: entry.name }));
      }
    };
    for (const { entry, file } of entries) {
      if (entry) await visit(entry, 0);
      else if (file && !stopped) {
        if (files.length < MAX_FILES) files.push(file);
        else { stopped = true; problems.push(limitMessage()); }
      }
    }
    return { files, problems };
  })();
}
