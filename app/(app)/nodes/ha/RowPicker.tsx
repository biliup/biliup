'use client'
import { Button, Checkbox, Tag, Typography } from '@douyinfe/semi-ui'
import type { LocalRow, RowsView } from '@/app/lib/fleet-ha'
import styles from './ha.module.scss'

const { Text } = Typography

export type Picked = { streamers: number[]; templates: number[] }

type Kind = keyof Picked

const KIND_LABEL: Record<Kind, string> = { streamers: '主播', templates: '投稿模板' }

/** 能勾的行：没有不能加入的原因、也还没在加入中 */
export function selectable(row: LocalRow): boolean {
  return !row.reason && row.state === 'local'
}

/** 缺省勾上服务端说缺省纳入的；上次被控制面退回的不再缺省勾上 */
export function defaultPicked(view: RowsView | null | undefined): Picked {
  const pick = (rows: LocalRow[] | undefined) =>
    (rows ?? []).filter((r) => r.included && selectable(r) && !r.refused).map((r) => r.id)
  return { streamers: pick(view?.streamers), templates: pick(view?.templates) }
}

export function pickedCount(picked: Picked): number {
  return picked.streamers.length + picked.templates.length
}

function rowNote(row: LocalRow): string | null {
  if (row.reason) return row.reason
  if (row.state === 'waiting') return row.busy ? '已经请它加入：等这一场录完、投完' : '已经请它加入：等它空闲一会儿就加入'
  if (row.state === 'joining') return '正在加入'
  if (row.busy) return '正在录，或上一场还没投完：这一场录完投完再加入'
  if (row.refused) return `上次没加进去：${row.refused}`
  return null
}

function PickRow({
  row,
  checked,
  disabled,
  onToggle,
}: {
  row: LocalRow
  checked: boolean
  disabled: boolean
  onToggle: (on: boolean) => void
}) {
  const note = rowNote(row)
  const off = !selectable(row)
  return (
    <li className={`${styles.pickRow} ${row.reason ? styles.pickOff : ''}`}>
      <Checkbox
        checked={checked || row.state !== 'local'}
        disabled={disabled || off}
        onChange={(e) => onToggle(Boolean(e.target.checked))}
        aria-label={row.name}
      >
        <span className={styles.pickText}>
          <Text strong ellipsis={{ showTooltip: true }}>
            {row.name || `#${row.id}`}
          </Text>
          {row.url ? (
            <Text type="tertiary" size="small" className={styles.pickReason}>
              {row.url}
            </Text>
          ) : null}
          {note ? <span className={styles.pickReason}>{note}</span> : null}
        </span>
      </Checkbox>
      {row.busy && !row.reason ? (
        <Tag size="small" color="orange">
          录完投完再加入
        </Tag>
      ) : null}
      {row.reason ? (
        <Tag size="small" color="grey">
          不能加入
        </Tag>
      ) : null}
    </li>
  )
}

/** 一台机器上还没纳入配对的主播与模板，可勾选；不能加入的灰掉并写原因 */
export function RowPicker({
  title,
  view,
  picked,
  onChange,
  disabled = false,
}: {
  title: string
  view: RowsView | null | undefined
  picked: Picked
  onChange: (picked: Picked) => void
  disabled?: boolean
}) {
  if (!view) return null
  if (view.error) {
    return (
      <section className={styles.pickGroup} aria-label={title}>
        <div className={styles.pickTitle}>{title}</div>
        <div className={styles.pickError}>{view.error}</div>
      </section>
    )
  }
  const groups = (['streamers', 'templates'] as Kind[]).map((kind) => ({ kind, rows: view[kind] ?? [] }))
  if (groups.every((g) => g.rows.length === 0)) {
    return (
      <section className={styles.pickGroup} aria-label={title}>
        <div className={styles.pickTitle}>{title}</div>
        <div className={styles.empty}>没有还没纳入配对的本地主播与模板</div>
      </section>
    )
  }
  const toggle = (kind: Kind, id: number, on: boolean) => {
    const rest = picked[kind].filter((x) => x !== id)
    onChange({ ...picked, [kind]: on ? [...rest, id] : rest })
  }
  return (
    <>
      {groups
        .filter((g) => g.rows.length > 0)
        .map(({ kind, rows }) => {
          const open = rows.filter(selectable)
          const all = open.length > 0 && open.every((r) => picked[kind].includes(r.id))
          return (
            <section key={kind} className={styles.pickGroup} aria-label={`${title}的${KIND_LABEL[kind]}`}>
              <div className={styles.pickTitle}>
                {title}的{KIND_LABEL[kind]}
                <Text type="tertiary" size="small">
                  勾了 {picked[kind].length} / 能加入 {open.length} / 共 {rows.length}
                </Text>
                {open.length > 1 ? (
                  <Button
                    size="small"
                    theme="borderless"
                    disabled={disabled}
                    onClick={() => onChange({ ...picked, [kind]: all ? [] : open.map((r) => r.id) })}
                  >
                    {all ? '全不选' : '全选'}
                  </Button>
                ) : null}
              </div>
              <ul className={styles.pickList}>
                {rows.map((row) => (
                  <PickRow
                    key={row.id}
                    row={row}
                    checked={picked[kind].includes(row.id)}
                    disabled={disabled}
                    onToggle={(on) => toggle(kind, row.id, on)}
                  />
                ))}
              </ul>
            </section>
          )
        })}
    </>
  )
}

function resultNote(row: LocalRow): { text: string | null; color: 'green' | 'grey' | 'orange' } {
  if (row.included) {
    return row.busy || row.state === 'waiting'
      ? { text: '这一场录完投完再加入', color: 'orange' }
      : { text: null, color: 'green' }
  }
  if (row.reason) return { text: row.reason, color: 'grey' }
  return { text: '没勾选，留作这台的本地行', color: 'grey' }
}

/** 纳入的结果：逐条写纳入与否、不能纳入的原因 */
export function AdoptionResult({ title, view }: { title: string; view: RowsView | null | undefined }) {
  if (!view) return null
  if (view.error) {
    return (
      <section className={styles.pickGroup} aria-label={title}>
        <div className={styles.pickTitle}>{title}</div>
        <div className={styles.pickError}>{view.error}</div>
      </section>
    )
  }
  const rows = [
    ...(view.streamers ?? []).map((row) => ({ row, kind: '主播' })),
    ...(view.templates ?? []).map((row) => ({ row, kind: '模板' })),
  ]
  return (
    <section className={styles.pickGroup} aria-label={title}>
      <div className={styles.pickTitle}>
        {title}
        <Text type="tertiary" size="small">
          纳入 {rows.filter((r) => r.row.included).length} / 共 {rows.length}
        </Text>
      </div>
      {rows.length === 0 ? (
        <div className={styles.empty}>这台上没有要纳入的本地行</div>
      ) : (
        <ul className={styles.pickList}>
          {rows.map(({ row, kind }) => {
            const note = resultNote(row)
            return (
              <li key={`${kind}-${row.id}`} className={styles.pickRow}>
                <span className={styles.pickText} style={{ flex: 1 }}>
                  <Text strong ellipsis={{ showTooltip: true }}>
                    {kind}「{row.name || `#${row.id}`}」
                  </Text>
                  {note.text ? <span className={styles.pickReason}>{note.text}</span> : null}
                </span>
                <Tag size="small" color={note.color} className={styles.result}>
                  {row.included ? '纳入' : '没纳入'}
                </Tag>
              </li>
            )
          })}
        </ul>
      )}
    </section>
  )
}
