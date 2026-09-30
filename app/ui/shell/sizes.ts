/**
 * 页面 / 抽屉 / 弹窗三种容器的归属规则（#1792，参考飞书项目）：
 * - 页面：一整套配置，或需要分区 / Tab 的长表单（约 20 个字段以上），要能刷新、分享链接、浏览器后退；
 * - 右侧抽屉：围绕列表里某一项的查看与编辑，背后的列表要留作上下文；
 * - 居中弹窗：一次性的短任务（不超过 5 个输入、确认、选择），做完回到原处；从抽屉里发起时也用弹窗，最多叠这一层。
 * 操作位置：
 * - 页面：主按钮在页头右上角，只放它；没有「取消」，返回靠页头左侧的返回箭头；不套单体大卡片，分区用标题与分隔线；
 * - 浮层（抽屉、弹窗）：底栏右下角「取消」+ 主按钮（实心；列表型抽屉的「+ 添加」用浅色带加号），「取消」只出现在浮层里；标题栏右侧只留关闭按钮；
 * - 主按钮写动作（创建模板 / 保存 / 添加…），不写「确定」。
 */

/** 弹窗三档：确认与 1–2 个输入 / 一般表单与结果 / 选择器、双栏、播放器 */
export const DIALOG_WIDTH = { sm: 480, md: 640, lg: 820 } as const
export type DialogSize = keyof typeof DIALOG_WIDTH

/** 抽屉两档，与 Semi SideSheet 的 small / medium 一致：列表型 / 表单型 */
export const SHEET_WIDTH = { sm: 448, md: 684 } as const
export type SheetSize = keyof typeof SHEET_WIDTH

/** 与 useIsMobile 的默认断点一致：不超过它时抽屉占满宽度、lg 弹窗全屏 */
export const NARROW = 640
