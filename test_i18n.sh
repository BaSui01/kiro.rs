#!/bin/bash

# i18n 功能测试脚本

echo "=========================================="
echo "Kiro Admin - 多语言功能测试"
echo "=========================================="
echo ""

# 检查翻译文件
echo "1. 检查翻译文件..."
if [ -f "admin-ui/src/i18n/zh-CN.json" ]; then
    echo "   ✅ zh-CN.json 存在"
else
    echo "   ❌ zh-CN.json 缺失"
fi

if [ -f "admin-ui/src/i18n/en-US.json" ]; then
    echo "   ✅ en-US.json 存在"
else
    echo "   ❌ en-US.json 缺失"
fi

if [ -f "admin-ui/src/i18n/ja-JP.json" ]; then
    echo "   ✅ ja-JP.json 存在"
else
    echo "   ❌ ja-JP.json 缺失"
fi

# 检查 i18n 配置
echo ""
echo "2. 检查 i18n 配置..."
if [ -f "admin-ui/src/i18n/index.ts" ]; then
    echo "   ✅ i18n 配置文件存在"
else
    echo "   ❌ i18n 配置文件缺失"
fi

# 检查语言切换器
echo ""
echo "3. 检查语言切换器..."
if [ -f "admin-ui/src/components/language-switcher.tsx" ]; then
    echo "   ✅ 语言切换器组件存在"
else
    echo "   ❌ 语言切换器组件缺失"
fi

# 检查依赖包
echo ""
echo "4. 检查依赖包..."
if grep -q "react-i18next" admin-ui/package.json; then
    echo "   ✅ react-i18next 已安装"
else
    echo "   ❌ react-i18next 未安装"
fi

if grep -q "i18next" admin-ui/package.json; then
    echo "   ✅ i18next 已安装"
else
    echo "   ❌ i18next 未安装"
fi

if grep -q "i18next-browser-languagedetector" admin-ui/package.json; then
    echo "   ✅ i18next-browser-languagedetector 已安装"
else
    echo "   ❌ i18next-browser-languagedetector 未安装"
fi

# 统计翻译键数量
echo ""
echo "5. 翻译键统计..."
zh_keys=$(grep -o '"[^"]*":' admin-ui/src/i18n/zh-CN.json | wc -l)
en_keys=$(grep -o '"[^"]*":' admin-ui/src/i18n/en-US.json | wc -l)
ja_keys=$(grep -o '"[^"]*":' admin-ui/src/i18n/ja-JP.json | wc -l)

echo "   中文翻译键: $zh_keys"
echo "   英文翻译键: $en_keys"
echo "   日文翻译键: $ja_keys"

# 检查编译状态
echo ""
echo "6. 检查编译状态..."
if [ -d "admin-ui/dist" ]; then
    echo "   ✅ 项目已编译"
    echo "   编译输出目录: admin-ui/dist/"
else
    echo "   ⚠️  项目未编译，运行 'cd admin-ui && npm run build'"
fi

echo ""
echo "=========================================="
echo "测试完成！"
echo "=========================================="
echo ""
echo "手动测试步骤："
echo "1. 运行 'cd admin-ui && npm run dev'"
echo "2. 打开浏览器访问 http://localhost:5173"
echo "3. 检查默认语言是否为中文"
echo "4. 使用语言切换器切换到英文"
echo "5. 使用语言切换器切换到日文"
echo "6. 刷新页面，确认语言选择被保留"
echo ""
