import type {Media} from './core/timeline/types'
import type {EditorDocument,MediaSource} from './project'

/** A re-shoot retains the media ID. Undo must restore that ID's file version as well. */
export class SourceBindings {
  private sources=new Map<string,MediaSource>()
  private key(media:Pick<Media,'id'|'path'>){return `${media.id}\0${media.path}`}
  remember(media:Media,source:MediaSource){this.sources.set(this.key(media),source)}
  document(document:EditorDocument){for(const media of document.project.media){const source=document.sources[media.id];if(source)this.remember(media,source)}}
  get(media:Media,candidate?:MediaSource):MediaSource {
    const matches=candidate?.kind==='file'?(media.path===`imports/file-${candidate.assetId}`||media.path===`imports/file-${candidate.assetId}.svg`):candidate?.kind==='recording'&&media.path.startsWith(`recordings/recording-${candidate.sessionId}-${candidate.segmentId}.`)
    if(candidate&&matches){this.remember(media,candidate);return candidate}
    const source=this.sources.get(this.key(media))
    if(!source)throw new Error(`素材 ${media.name} 的文件版本未登记，请重新打开工程`)
    return source
  }
}
