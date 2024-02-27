"use client";
import React from "react";
import styles from "../../styles/dashboard.module.scss";
import { Form, Select, Space } from "@douyinfe/semi-ui";
import { IconSetting } from "@douyinfe/semi-icons";

const Developer: React.FC = () => {
    return (
        <>
            <div className={styles.frameDeveloper}>
                <div className={styles.frameInside}>
                    <div className={styles.group}>
                        <div className={styles.buttonOnlyIconSecond} />
                        <div
                            className={styles.lineStory}
                            style={{
                                color: "var(--semi-color-bg-0)",
                                display: "flex",
                            }}
                        >
                            <IconSetting size="small" />
                        </div>
                    </div>
                    <p className={styles.meegoSharedWebWorkIt}>开发者选项</p>
                </div>
                <Form.Select
                    label=" 控制台日志输出等级（LOGGING.root.level）"
                    field="LOGGING.root.level"
                    placeholder="INFO"
                    maxTagCount={3}
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                >
                    <Select.Option value="DEBUG">DEBUG</Select.Option>
                    <Select.Option value="INFO">INFO</Select.Option>
                    <Select.Option value="WARNING">WARNING</Select.Option>
                    <Select.Option value="ERROR">ERROR</Select.Option>
                    <Select.Option value="CRITICAL">CRITICAL</Select.Option>
                </Form.Select>
                <Form.Select
                    label=" 文件日志输出等级（LOGGING.loggers.biliup.level）"
                    field="LOGGING.loggers.biliup.level"
                    placeholder="INFO"
                    maxTagCount={3}
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                >
                    <Select.Option value="DEBUG">DEBUG</Select.Option>
                    <Select.Option value="INFO">INFO</Select.Option>
                    <Select.Option value="WARNING">WARNING</Select.Option>
                    <Select.Option value="ERROR">ERROR</Select.Option>
                    <Select.Option value="CRITICAL">CRITICAL</Select.Option>
                </Form.Select>
            </div>
        </>
    );
};

export default Developer;
