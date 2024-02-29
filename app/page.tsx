"use client";
import {
    Layout,
    Nav,
    Button,
    Breadcrumb,
    Skeleton,
    Avatar,
    Tag,
    Modal,
    Form,
    Row,
    Col,
    Dropdown,
    SplitButtonGroup,
    Typography,
    Popconfirm,
    List,
    Descriptions,
    Rating,
    ButtonGroup,
} from "@douyinfe/semi-ui";

import {
    IconBell,
    IconHelpCircle,
    IconBytedanceLogo,
    IconPlusCircle,
    IconHistogram,
    IconLive,
    IconSetting,
    IconStoryStroked,
    IconCheckCircleStroked,
    IconVideoListStroked,
    IconTreeTriangleDown,
    IconSendStroked,
    IconEdit2Stroked,
    IconDeleteStroked,
} from "@douyinfe/semi-icons";
import { useState } from "react";
import useStreamers from "./lib/use-streamers";
import TemplateModal from "./ui/TemplateModal";
import { DropDownMenuItem } from "@douyinfe/semi-ui/lib/es/dropdown";
import { LiveStreamerEntity } from "./lib/api-streamer";

const Home: React.FC = () => (
    <iframe style={{
        borderWidth: 0
    }} height="100%" src='https://biliup.github.io/biliup/docs/guide/changelog/'></iframe>
);

export default Home;
