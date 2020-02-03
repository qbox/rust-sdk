RSpec.describe QiniuNg::Bindings do
  context QiniuNg::Bindings::Str do
    it 'should be ok to initialize string' do
      str1 = QiniuNg::Bindings::Str.new '你好'
      str2 = QiniuNg::Bindings::Str.new '七牛'
      expect(str1.get_ptr).to eq('你好')
      expect(str2.get_ptr).to eq('七牛')
      expect(str1.get_len).to eq('你好'.bytesize)
      expect(str2.get_len).to eq('七牛'.bytesize)
      expect(str1.is_freed?).to be false
      expect(str2.is_freed?).to be false
      expect(str1.is_null?).to be false
      expect(str2.is_null?).to be false
    end
  end

  context QiniuNg::Bindings::StrList do
    it 'should be ok to initialize string list' do
      list1 = QiniuNg::Bindings::StrList.new(['七牛', '你好', '武汉', '加油'])
      list2 = QiniuNg::Bindings::StrList.new(['科多兽', '多啦A梦', '潘多拉'])
      expect(list1.len).to eq(4)
      expect(list2.len).to eq(3)
      expect(list1.get(0)).to eq('七牛')
      expect(list1.get(1)).to eq('你好')
      expect(list1.get(2)).to eq('武汉')
      expect(list1.get(3)).to eq('加油')
      expect(list2.get(0)).to eq('科多兽')
      expect(list2.get(1)).to eq('多啦A梦')
      expect(list2.get(2)).to eq('潘多拉')
      expect(list1.is_freed?).to be false
      expect(list2.is_freed?).to be false
    end
  end

  context QiniuNg::Bindings::StrMap do
    it 'should be ok to initialize string map' do
      map1 = QiniuNg::Bindings::StrMap.new 5
      map1.set('KODO', '科多兽')
      map1.set('多啦A梦', 'DORA')
      map1.set('PANDORA', '潘多拉')

      map2 = QiniuNg::Bindings::StrMap.new 10
      map2.set('科多兽', 'KODO')
      map2.set('DORA', '多啦A梦')
      map2.set('潘多拉', 'PANDORA')

      expect(map1.len).to eq(3)
      expect(map1.get('KODO')).to eq('科多兽')
      expect(map1.get('多啦A梦')).to eq('DORA')
      expect(map1.get('PANDORA')).to eq('潘多拉')

      expect(map2.len).to eq(3)
      expect(map2.get('科多兽')).to eq('KODO')
      expect(map2.get('DORA')).to eq('多啦A梦')
      expect(map2.get('潘多拉')).to eq('PANDORA')

      looped = 0
      map1.each_entry(->(key, value, _) do
        case key
        when 'KODO' then
          expect(value).to eq('科多兽')
        when '多啦A梦' then
          expect(value).to eq('DORA')
        when 'PANDORA' then
          expect(value).to eq('潘多拉')
        else
          fail "Unrecognized key: #{key}"
        end
        looped += 1
        true
      end, FFI::MemoryPointer.new(:pointer))
      expect(looped).to eq 3
    end
  end
end
