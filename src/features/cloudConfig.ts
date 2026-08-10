/**
 * 正式安装包在构建时注入公网 HTTPS 后端；开发环境仍可回退到本机服务。
 * 用户手动修改后的地址保存在 localStorage，优先级高于此默认值。
 */
export const DEFAULT_CLOUD_BASE_URL=(import.meta.env.VITE_TOKEN_MANAGER_CLOUD_BASE_URL||'https://token-manager-cloud.netlify.app').replace(/\/$/,'')

/** 已安装的旧版本曾默认保存本机测试地址，正式版首次启动时自动迁移到公网。 */
export function currentCloudBaseUrl(){
 const stored=localStorage.getItem('token-manager-cloud-base-url')?.replace(/\/$/,'')||''
 const legacyLocal=/^http:\/\/(127\.0\.0\.1|localhost):8787$/i.test(stored)
 if(!stored||(import.meta.env.PROD&&legacyLocal&&DEFAULT_CLOUD_BASE_URL.startsWith('https://'))){
  localStorage.setItem('token-manager-cloud-base-url',DEFAULT_CLOUD_BASE_URL)
  return DEFAULT_CLOUD_BASE_URL
 }
 return stored
}
