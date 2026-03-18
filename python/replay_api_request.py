#!/usr/bin/env python3
"""
API 请求重放测试脚本

用于重放从 api.log 中复制的 Request payload，测试是否因敏感内容导致空响应。

使用方法:
    1. 设置 API_BASE 和 API_KEY 环境变量
    2. 运行脚本
    3. 粘贴从 api.log 复制的 Request payload JSON

示例:
    export API_BASE="https://api.openai.com/v1"
    export API_KEY="sk-xxx"
    python replay_api_request.py
"""

import asyncio
import httpx
import json
import os
import sys


def get_input_multiline(prompt: str = "") -> str:
    """读取多行输入，以空行结束"""
    print(prompt)
    print("(粘贴 JSON payload，输入完成后按 Ctrl+D 或输入空行结束)")
    lines = []
    try:
        while True:
            line = input()
            if line == "":
                break
            lines.append(line)
    except EOFError:
        pass
    return "\n".join(lines)


def detect_api_format(payload: dict) -> str:
    """根据 payload 内容检测 API 格式"""
    if "messages" in payload:
        return "chat_completions"
    elif "input" in payload or "instructions" in payload:
        return "responses"
    else:
        return "unknown"


def build_url_and_payload(api_base: str, payload: dict, api_format: str) -> tuple[str, dict]:
    """根据 API 格式构建 URL 和 payload"""
    base = api_base.rstrip("/")

    if api_format == "chat_completions":
        url = f"{base}/chat/completions"
        return url, payload
    elif api_format == "responses":
        url = f"{base}/v1/responses"
        return url, payload
    else:
        raise ValueError(f"Unknown API format: {api_format}")


async def send_request(api_base: str, api_key: str, payload: dict) -> dict:
    """发送 API 请求并返回详细信息"""

    # 检测 API 格式
    api_format = detect_api_format(payload)
    print(f"\n检测到 API 格式: {api_format}")

    # 构建 URL
    url, final_payload = build_url_and_payload(api_base, payload, api_format)

    # Headers
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
    }

    result = {
        "url": url,
        "api_format": api_format,
        "status_code": None,
        "success": False,
        "has_content": False,
        "content_length": 0,
        "error": None,
        "response_data": None,
    }

    print(f"\n请求 URL: {url}")
    print(f"Payload 长度: {len(json.dumps(final_payload, ensure_ascii=False))} chars")
    print(f"Model: {final_payload.get('model', 'N/A')}")

    try:
        async with httpx.AsyncClient(timeout=120.0) as client:
            response = await client.post(url, json=final_payload, headers=headers)
            result["status_code"] = response.status_code

            print(f"\nHTTP Status: {response.status_code}")

            if response.status_code != 200:
                result["error"] = f"HTTP {response.status_code}: {response.text[:500]}"
                print(f"❌ 错误响应: {response.text[:500]}")
                return result

            response_json = response.json()
            result["response_data"] = response_json

            # 提取内容
            content = ""

            if api_format == "chat_completions":
                choices = response_json.get("choices", [])
                if choices:
                    message = choices[0].get("message", {})
                    content = message.get("content", "")
                    result["finish_reason"] = choices[0].get("finish_reason")
                    print(f"Finish Reason: {result.get('finish_reason')}")

            elif api_format == "responses":
                # Responses API 格式
                for item in response_json.get("output", []):
                    if item.get("type") == "message":
                        for c in item.get("content", []):
                            if c.get("type") in ("output_text", "text"):
                                content += c.get("text", "")
                # 兼容旧格式
                if not content and "output_text" in response_json:
                    content = response_json.get("output_text", "")

                result["status"] = response_json.get("status")
                print(f"Status: {response_json.get('status')}")

            result["content_length"] = len(content)
            result["has_content"] = bool(content.strip())

            usage = response_json.get("usage", {})
            print(f"\nToken 使用:")
            print(f"  Input:  {usage.get('input_tokens', 0)}")
            print(f"  Output: {usage.get('output_tokens', 0)}")
            print(f"  Total:  {usage.get('total_tokens', 0)}")

            print(f"\n响应内容长度: {len(content)} chars")

            if content.strip():
                result["success"] = True
                print(f"✅ 有内容!")
                print(f"\n内容预览 (前 500 字符):")
                print("-" * 60)
                print(content[:500])
                if len(content) > 500:
                    print(f"\n... (还有 {len(content) - 500} 字符)")
            else:
                print(f"❌ 响应为空!")
                # 检查可能的过滤原因
                if response_json.get("status") == "cancelled":
                    result["error"] = "Response was cancelled (可能触发内容过滤)"
                elif "error" in response_json:
                    result["error"] = response_json.get("error")

    except httpx.TimeoutException as e:
        result["error"] = f"Timeout: {e}"
        print(f"❌ 超时: {e}")
    except httpx.ConnectError as e:
        result["error"] = f"Connection Error: {e}"
        print(f"❌ 连接失败: {e}")
    except Exception as e:
        result["error"] = f"Exception: {e}"
        print(f"❌ 异常: {e}")
        import traceback
        traceback.print_exc()

    return result


def print_summary(result: dict):
    """打印测试摘要"""
    print("\n" + "=" * 60)
    print("测试摘要")
    print("=" * 60)
    print(f"API 格式: {result['api_format']}")
    print(f"HTTP 状态: {result['status_code']}")
    print(f"是否有内容: {'✅ 是' if result['has_content'] else '❌ 否'}")
    print(f"内容长度: {result['content_length']} chars")

    if result['error']:
        print(f"\n错误: {result['error']}")

    print("\n结论:")
    if result['success']:
        print("✅ 请求成功，API 返回了内容")
    elif result['status_code'] == 200 and not result['has_content']:
        print("⚠️ HTTP 200 但内容为空，这可能意味着:")
        print("   - API 触发了内容安全过滤")
        print("   - JSON schema 解析失败")
        print("   - 模型拒绝生成内容")
    elif result['status_code'] != 200:
        print(f"❌ HTTP 错误 ({result['status_code']}), 可能是认证或配置问题")


def main():
    print("=" * 60)
    print("API 请求重放测试脚本")
    print("=" * 60)

    # 获取配置
    api_base = "http://localhost:8317"
    api_key = "xc-2025"

    print(f"\n当前配置:")
    print(f"  API_BASE: {api_base}")
    print(f"  API_KEY: {'*' * 20 + api_key[-4:] if len(api_key) > 4 else '未设置'}")

    if not api_key or api_key == "your-api-key":
        print("\n❌ 请设置 API_KEY 环境变量")
        print("   示例: export API_KEY='sk-xxx'")
        sys.exit(1)

    # 允许用户修改配置
    try:
        new_base = input(f"\nAPI_BASE [{api_base}]: ").strip()
        if new_base:
            api_base = new_base

        new_key = input(f"API_KEY [已设置]: ").strip()
        if new_key:
            api_key = new_key
    except EOFError:
        pass

    print("\n" + "=" * 60)
    print("请粘贴从 api.log 复制的 Request payload JSON")
    print("=" * 60)
    print("\n日志格式示例:")
    print('[2025-01-28 12:00:00] [INFO] [Python] [LLM:Responses] Request payload: {"model":"gpt-4o-mini","input":"...","instructions":"..."}')
    print("\n只需复制冒号后面的 JSON 部分，即:")
    print('{"model":"gpt-4o-mini","input":"...","instructions":"..."}')

    # 读取 payload
    payload_str = get_input_multiline("\n请粘贴 payload JSON:")

    if not payload_str.strip():
        print("\n❌ 未输入 payload")
        sys.exit(1)

    try:
        payload = json.loads(payload_str)
    except json.JSONDecodeError as e:
        print(f"\n❌ JSON 解析失败: {e}")
        print(f"\n输入内容:\n{payload_str[:500]}")
        sys.exit(1)

    print(f"\n✅ JSON 解析成功")
    print(f"  Model: {payload.get('model', 'N/A')}")
    print(f"  Keys: {list(payload.keys())}")

    # 确认发送
    try:
        confirm = input("\n是否发送请求? (y/n, 默认 y): ").strip().lower()
        if confirm and confirm != 'y':
            print("已取消")
            sys.exit(0)
    except EOFError:
        pass

    # 发送请求
    result = asyncio.run(send_request(api_base, api_key, payload))

    # 打印摘要
    print_summary(result)

    # 保存结果
    output_file = "replay_result.json"
    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)
    print(f"\n详细结果已保存到: {output_file}")


if __name__ == "__main__":
    main()
