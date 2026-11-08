//! Reference contact sheets use registered brand files and the existing FFmpeg/export compositor.
use super::super::{self as editor, Error, text_model::conversation::Image};
use super::{
    service::{RunContext, Service},
    views,
};
use crate::server::common::processing::{engine, transcode};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::time::Duration;
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

impl Service {
    pub(super) async fn brand_reference(
        &self,
        scope: super::Turn<'_>,
        revision: i64,
        context: &mut RunContext,
        state: &Value,
        args: &Value,
    ) -> Result<Value, Error> {
        let super::Turn {
            project,
            id: turn,
            cancel,
        } = scope;
        let source = args["path"].as_str().unwrap();
        let id = args["kit"]
            .as_str()
            .or_else(|| state["active"]["kit"].as_str());
        let selected = state["kits"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|kit| kit["id"].as_str() == id);
        let files = editor::resources::files(&self.pool, project).await?;
        let file = files
            .iter()
            .find(|file| {
                file["url"] == source
                    || selected.is_some_and(|kit| {
                        file["kit"] == kit["id"]
                            && file["scope"] == kit["scope"]
                            && file["name"] == source
                    })
            })
            .ok_or_else(|| {
                invalid("参考文件未登记，请先从品牌面板选择文件，或使用 brand_read 返回的资源链接")
            })?;
        let name = file["name"].as_str().unwrap();
        let scope = file["scope"].as_str().unwrap();
        let kit = file["kit"].as_str().unwrap();
        let extension = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        if matches!(
            extension.as_str(),
            "svg" | "png" | "jpg" | "jpeg" | "webp" | "gif"
        ) {
            let (image, url) = self
                .brand_picture(
                    super::Turn {
                        project,
                        id: turn,
                        cancel,
                    },
                    revision,
                    scope,
                    kit,
                    name,
                    "mark",
                )
                .await?;
            context.images.push(image);
            return Ok(
                json!({"file":name,"image":url,"evidence":"品牌参考原图，非时间轴渲染","count":1}),
            );
        }
        if !matches!(extension.as_str(), "mp4" | "mov" | "webm") {
            return Err(invalid("参考画面仅支持已登记的图片或视频"));
        }
        let path =
            editor::resources::file(&self.pool, &self.root, project, scope, kit, name).await?;
        let child = cancel.child_token();
        let _guard = child.clone().drop_guard();
        let count = args["count"].as_u64().unwrap_or(16) as usize;
        let (jpeg,times,seconds)=tokio::time::timeout(Duration::from_secs(120),async{
            let permit=transcode::acquire(&child).await.map_err(invalid)?;
            let mut command=crate::tools::ffmpeg_command();
            command.args(["-hide_banner","-nostdin","-protocol_whitelist","file,pipe","-format_whitelist",editor::imports::FORMATS,"-i"]).arg(&path)
                .args(["-map","0:v:0","-frames:v","1","-t","0.05","-an","-sn","-dn","-f","null","-"]);
            let log=engine::run_process(command,&child,Duration::from_secs(30),|_,_|{}).await.map_err(invalid)?;drop(permit);
            let (kind,duration,_,_)=editor::imports::metadata(&log).map_err(invalid)?;
            if kind!="video"{return Err(invalid("参考文件不是可读取的视频"))}
            let seconds=duration.filter(|ms|*ms>0).ok_or_else(||invalid("无法确定参考视频时长"))? as f64/1000.;
            let directory=tempfile::tempdir().map_err(|e|invalid(e.to_string()))?;
            let mut cells=vec![];let mut times=vec![];
            for index in 0..count{
                let at=seconds*index as f64/count as f64;times.push(at);
                let output=directory.path().join(format!("{index}.jpg"));
                let _permit=transcode::acquire(&child).await.map_err(invalid)?;
                let mut command=crate::tools::ffmpeg_command();
                command.args(["-hide_banner","-nostdin","-y","-threads","1","-protocol_whitelist","file,pipe","-format_whitelist",editor::imports::FORMATS,"-ss",&format!("{at:.6}"),"-i"]).arg(&path)
                    .args(["-map","0:v:0","-frames:v","1","-an","-sn","-dn","-vf","scale=320:180:force_original_aspect_ratio=decrease,pad=320:180:(ow-iw)/2:(oh-ih)/2,setsar=1","-c:v","mjpeg","-q:v","3","-threads","1","-update","1"]).arg(&output);
                engine::run_process(command,&child,Duration::from_secs(30),|_,_|{}).await.map_err(invalid)?;
                let bytes=editor::resources::importer::read(&output,2*1024*1024).await?;
                cells.push(json!({"jpeg":STANDARD.encode(bytes),"frame":(at*1000.).round() as u64,"sourceSeconds":at}));
            }
            let cols=count.min(4);let rows=count.div_ceil(cols);
            let grid=json!({"columns":cols,"rows":rows,"cellWidth":320,"cellHeight":180,"width":cols*326+6,"height":rows*208+6});
            let jpeg=editor::export::frames::sheet(&json!(cells),&json!(1000),&grid,&child).await.map_err(invalid)?;
            Ok::<_,Error>((jpeg,times,seconds))
        }).await.map_err(|_|invalid("参考视频取帧超时，请减少帧数后重试"))??;
        if cancel.is_cancelled() {
            return Err(invalid("助手已停止"));
        }
        let url = views::keep(&self.pool, project, turn, revision, &jpeg).await?;
        context.images.push(Image {
            name: format!("参考视频 {name} 联系表（非时间轴）"),
            mime_type: "image/jpeg".into(),
            data: STANDARD.encode(jpeg),
        });
        Ok(
            json!({"file":name,"image":url,"count":count,"seconds":seconds,"timestampsSeconds":times,"evidence":"原始参考视频，非时间轴渲染","next":"仅根据实际取帧描述参考风格，不要把参考视频中的品牌配色或字体自动替换为工程品牌。"}),
        )
    }
}
