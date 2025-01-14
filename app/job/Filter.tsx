import { AutoComplete } from '@douyinfe/semi-ui'
import { IconSearch } from '@douyinfe/semi-icons'

const Filter = (props: any) => {
  console.log('renderFilterDropdown', props)
  const { tempFilteredValue, setTempFilteredValue, confirm, clear, close } = props

  const handleChange = (value: any) => {
    const filteredValue = value ? [value] : []
    setTempFilteredValue(filteredValue)
    // 你也可以在 input value 变化时直接筛选
    confirm({ filteredValue })
  }

  return (
    <AutoComplete
      value={tempFilteredValue[0]}
      showClear
      prefix={<IconSearch />}
      placeholder="搜索... "
      onChange={handleChange}
      style={{ width: 200, margin: 5 }}
    />
  )
}

export default Filter
