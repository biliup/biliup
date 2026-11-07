//! Provider-neutral conversation and function-call transport. No desktop CLI or shell tools.
use super::Settings;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashSet, time::Duration};
use tokio_util::sync::CancellationToken;

const MAX_INPUT: usize = 32 * 1024 * 1024;
const MAX_OUTPUT: usize = 8 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Image {
    pub name: String,
    pub mime_type: String,
    pub data: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Call {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}
pub struct Reply {
    pub text: String,
    pub calls: Vec<Call>,
    pub input_tokens: u64,
    pub output_tokens: u64,
}
pub struct Conversation {
    settings: Settings,
    instructions: String,
    tools: Vec<Tool>,
    items: Vec<Value>,
    pending: Vec<(Call, Option<String>)>,
    requests: u32,
    client: reqwest::Client,
    effort: Option<String>,
}
impl Conversation {
    pub fn new(settings: Settings, instructions: &str, tools: Vec<Tool>) -> Result<Self, String> {
        super::validate(&settings)?;
        if instructions.len() > 1024 * 1024 || tools.len() > 64 {
            return Err("助手指令或工具数量超出限制".into());
        }
        let mut names = HashSet::new();
        for tool in &tools {
            if tool.name.is_empty()
                || tool.name.len() > 64
                || !tool
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                || !names.insert(tool.name.clone())
                || tool.description.len() > 32000
                || !tool.input_schema.is_object()
            {
                return Err("助手工具定义无效".into());
            }
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(240))
            .build()
            .map_err(|_| "无法创建模型客户端")?;
        Ok(Self {
            settings,
            instructions: instructions.into(),
            tools,
            items: vec![],
            pending: vec![],
            requests: 0,
            client,
            effort: None,
        })
    }
    pub fn set_effort(&mut self, effort: Option<&str>) -> Result<(), String> {
        if let Some(value) = effort {
            if !matches!(value, "low" | "medium" | "high" | "max") {
                return Err("推理强度无效".into());
            }
            if self.settings.provider == "google" && value == "max" {
                return Err("Google 模型的最高推理强度为 high".into());
            }
        }
        self.effort = effort.map(str::to_string);
        Ok(())
    }
    /// Visible messages only. Native reasoning/signature items from this run remain opaque below.
    pub fn message(&mut self, role: &str, text: &str, images: &[Image]) -> Result<(), String> {
        if !self.pending.is_empty() {
            return Err("请先提交工具调用结果".into());
        }
        if !["user", "assistant"].contains(&role)
            || text.len() > 1024 * 1024
            || images.len() > 8
            || role == "assistant" && !images.is_empty()
        {
            return Err("助手消息无效或过大".into());
        }
        let mut bytes = 0;
        for image in images {
            if !["image/png", "image/jpeg", "image/webp", "image/gif"]
                .contains(&image.mime_type.as_str())
                || image.data.len() > 8 * 1024 * 1024
            {
                return Err("附件图片类型或大小无效".into());
            }
            let decoded = STANDARD
                .decode(&image.data)
                .map_err(|_| "附件图片编码无效")?;
            bytes += decoded.len();
            if decoded.is_empty() || decoded.len() > 5 * 1024 * 1024 || bytes > 20 * 1024 * 1024 {
                return Err("单张图片上限 5 MiB，附件合计上限 20 MiB".into());
            }
        }
        let value = if self.settings.provider == "openai" && role == "assistant" {
            json!({"role":"assistant","content":text})
        } else if self.settings.provider == "openai" {
            let mut content =
                vec![json!({"type":if role=="user"{"input_text"}else{"output_text"},"text":text})];
            content.extend(images.iter().map(|image|json!({"type":"input_image","image_url":format!("data:{};base64,{}",image.mime_type,image.data)})));
            json!({"role":role,"content":content})
        } else {
            let mut parts = vec![json!({"text":text})];
            parts.extend(
                images.iter().map(
                    |image| json!({"inlineData":{"mimeType":image.mime_type,"data":image.data}}),
                ),
            );
            json!({"role":if role=="assistant"{"model"}else{"user"},"parts":parts})
        };
        self.items.push(value);
        Ok(())
    }
    /// Require one result per call; never let duplicate or missing outputs corrupt the tool chain.
    pub fn results(&mut self, results: Vec<(String, Value)>) -> Result<(), String> {
        if results.len() != self.pending.len() || self.pending.is_empty() {
            return Err("工具结果数量不匹配".into());
        }
        let ids: HashSet<_> = results.iter().map(|(id, _)| id).collect();
        if ids.len() != results.len()
            || self.pending.iter().any(|(call, _)| !ids.contains(&call.id))
        {
            return Err("工具结果标识不匹配".into());
        }
        if serde_json::to_vec(&results)
            .map_err(|_| "工具结果格式无效")?
            .len()
            > MAX_OUTPUT
        {
            return Err("工具结果过大，请缩小查询范围".into());
        }
        let mut values = Vec::new();
        for (call, native_id) in &self.pending {
            let result = &results.iter().find(|(id, _)| id == &call.id).unwrap().1;
            if self.settings.provider == "openai" {
                values.push(json!({"type":"function_call_output","call_id":call.id,"output":result.to_string()}));
            } else {
                let mut response = json!({"name":call.name,"response":if result.is_object(){result.clone()}else{json!({"result":result})}});
                if let Some(id) = native_id {
                    response["id"] = json!(id)
                }
                values.push(json!({"functionResponse":response}));
            }
        }
        if self.settings.provider == "google" {
            self.items.push(json!({"role":"user","parts":values}))
        } else {
            self.items.extend(values)
        }
        self.pending.clear();
        Ok(())
    }
    pub async fn next(&mut self, key: &str, cancel: &CancellationToken) -> Result<Reply, String> {
        if !self.pending.is_empty() {
            return Err("上一批工具调用尚未完成".into());
        }
        if self.requests >= 32 {
            return Err("本轮助手已达到 32 次模型请求上限".into());
        }
        if self.items.is_empty() {
            return Err("助手消息不能为空".into());
        }
        let (url, header, credential, mut body) = if self.settings.provider == "openai" {
            (
                "https://api.openai.com/v1/responses".to_string(),
                "Authorization",
                format!("Bearer {key}"),
                json!({"model":self.settings.model,"instructions":self.instructions,"input":self.items,"store":false,"include":["reasoning.encrypted_content"],"max_output_tokens":16384}),
            )
        } else {
            (
                format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                    self.settings.model
                ),
                "x-goog-api-key",
                key.into(),
                json!({"systemInstruction":{"parts":[{"text":self.instructions}]},"contents":self.items,"generationConfig":{"maxOutputTokens":16384}}),
            )
        };
        if let Some(effort) = &self.effort {
            if self.settings.provider == "openai" {
                body["reasoning"] = json!({"effort":effort});
            } else {
                body["generationConfig"]["thinkingConfig"] =
                    json!({"thinkingLevel":effort.to_uppercase()});
            }
        }
        if !self.tools.is_empty() {
            body["tools"] = if self.settings.provider == "openai" {
                json!(self.tools.iter().map(|tool|json!({"type":"function","name":tool.name,"description":tool.description,"parameters":tool.input_schema,"strict":false})).collect::<Vec<_>>())
            } else {
                json!([{"functionDeclarations":self.tools.iter().map(|tool|json!({"name":tool.name,"description":tool.description,"parametersJsonSchema":tool.input_schema})).collect::<Vec<_>>() }])
            };
        }
        let bytes = serde_json::to_vec(&body).map_err(|_| "模型请求格式无效")?;
        if bytes.len() > MAX_INPUT {
            return Err("会话上下文过大，请新建聊天或减少附件".into());
        }
        self.requests += 1;
        let request = async {
            let mut response = self
                .client
                .post(url)
                .header(header, credential)
                .header("Content-Type", "application/json")
                .body(bytes)
                .send()
                .await
                .map_err(|_| "模型请求未完成，请检查供应商记录后重试")?;
            if !response.status().is_success() {
                return Err(format!(
                    "模型请求失败（HTTP {}），请检查模型名称、密钥和额度",
                    response.status().as_u16()
                ));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| "模型响应中断")? {
                if bytes.len() + chunk.len() > MAX_OUTPUT {
                    return Err("模型响应过大".into());
                }
                bytes.extend_from_slice(&chunk)
            }
            serde_json::from_slice::<Value>(&bytes).map_err(|_| "模型响应格式无效".into())
        };
        let response = tokio::select! {_=cancel.cancelled()=>return Err("助手已停止".into()),result=request=>result?};
        self.accept(response)
    }
    fn accept(&mut self, response: Value) -> Result<Reply, String> {
        let mut text = Vec::new();
        let mut calls = Vec::new();
        let mut pending = Vec::new();
        let mut accept_call =
            |name: &Value, args: Value, id: Option<&str>, index: usize| -> Result<(), String> {
                let name = name.as_str().ok_or("模型工具调用缺少名称")?;
                if !self.tools.iter().any(|tool| tool.name == name) || !args.is_object() {
                    return Err("模型返回未声明的工具或无效参数".into());
                }
                let call = Call {
                    id: id
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("{}-{index}", self.requests)),
                    name: name.into(),
                    arguments: args,
                };
                if calls.iter().any(|prior: &Call| prior.id == call.id) {
                    return Err("模型返回重复工具调用标识".into());
                }
                pending.push((call.clone(), id.map(str::to_owned)));
                calls.push(call);
                Ok(())
            };
        let (native, input_tokens, output_tokens) = if self.settings.provider == "openai" {
            if response["status"] != "completed" {
                return Err("模型未完成本次响应，本次工具未执行".into());
            }
            let output = response["output"].as_array().ok_or("模型未返回内容")?;
            for (index, item) in output.iter().enumerate() {
                match item["type"].as_str() {
                    Some("message") => {
                        for part in item["content"].as_array().into_iter().flatten() {
                            if part["type"] == "output_text" {
                                if let Some(value) = part["text"].as_str() {
                                    text.push(value.to_string())
                                }
                            } else if part["type"] == "refusal"
                                && let Some(value) = part["refusal"].as_str()
                            {
                                text.push(value.to_string())
                            }
                        }
                    }
                    Some("function_call") => {
                        let id = item["call_id"]
                            .as_str()
                            .filter(|id| !id.is_empty())
                            .ok_or("模型调用缺少标识")?;
                        let args = serde_json::from_str(
                            item["arguments"].as_str().ok_or("模型调用参数缺失")?,
                        )
                        .map_err(|_| "模型调用参数不是有效 JSON")?;
                        accept_call(&item["name"], args, Some(id), index)?
                    }
                    _ => {}
                }
            }
            (
                output.clone(),
                response["usage"]["input_tokens"].as_u64().unwrap_or(0),
                response["usage"]["output_tokens"].as_u64().unwrap_or(0),
            )
        } else {
            let candidate = &response["candidates"][0];
            if candidate["finishReason"] != "STOP" {
                return Err("模型未完成本次响应，本次工具未执行".into());
            }
            let content = &candidate["content"];
            for (index, part) in content["parts"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
            {
                if part["thought"] != true
                    && let Some(value) = part["text"].as_str()
                {
                    text.push(value.into())
                }
                if let Some(call) = part.get("functionCall") {
                    accept_call(
                        &call["name"],
                        call.get("args").cloned().unwrap_or_else(|| json!({})),
                        call["id"].as_str(),
                        index,
                    )?
                }
            }
            (
                vec![content.clone()],
                response["usageMetadata"]["promptTokenCount"]
                    .as_u64()
                    .unwrap_or(0),
                response["usageMetadata"]["candidatesTokenCount"]
                    .as_u64()
                    .unwrap_or(0),
            )
        };
        if calls.len() > 16 {
            return Err("模型一次返回过多工具调用，本次工具未执行".into());
        }
        let text = text.join("\n");
        if text.trim().is_empty() && calls.is_empty() {
            return Err("模型没有返回可用内容".into());
        }
        // Preserve OpenAI encrypted reasoning items and Gemini thought signatures exactly.
        // They are provider context, never text shown to the user or tool instructions.
        self.items.extend(native);
        self.pending = pending;
        Ok(Reply {
            text,
            calls,
            input_tokens,
            output_tokens,
        })
    }
}
