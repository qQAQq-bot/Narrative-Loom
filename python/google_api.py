from google import genai
from google.oauth2 import service_account
from PIL import Image
import os
import logging
import httpx

# ---------------------- 基础配置（核心，必须修改）----------------------
API_KEY = "AIzaSyBstJ5Q2Q0UK9-NclkqxDtEKMoyWtji7EE"
PROXY_URL = "http://127.0.0.1:7890"  # 修改为你的代理地址

# ---------------------- 日志配置（可选，便于调试）----------------------
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S"
)
logger = logging.getLogger(__name__)

# ---------------------- 初始化Gemini客户端----------------------
def init_gemini_client():
    """初始化Gemini客户端，返回模型实例"""
    try:
        client = genai.Client(
            api_key=API_KEY,
            http_options={
                'timeout': 60000,
                'client_args': {
                    'proxy': PROXY_URL
                }
            }
        )
        logger.info("Gemini 客户端初始化成功")
        return client
    except Exception as e:
        logger.error(f"客户端初始化失败：{str(e)}", exc_info=True)
        raise



if __name__ == "__main__":
    client = init_gemini_client()

    # model_names = [m.name.replace("models/", "") for m in client.models.list() if m.name]
    # print(",".join(sorted(model_names)))

    response = client.models.embed_content(
        model='text-embedding-004',
        contents=['你好世界', '机器学习']
    )
    # 获取向量
    embeddings = response.embeddings
    for emb in embeddings:
        print(emb.values)  # 向量数组