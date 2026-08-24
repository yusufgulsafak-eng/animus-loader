export interface RegistrationInput {displayName:string;email:string;password:string;passwordConfirm:string}
export function validateRegistration(input:RegistrationInput):string|null{
  const name=input.displayName.trim();const email=input.email.trim();
  if(name.length<2||name.length>100)return "Görünen ad 2-100 karakter olmalıdır.";
  if(!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email))return "Geçerli bir e-posta adresi girin.";
  if(input.password.length<12||!/[A-Z]/.test(input.password)||!/[a-z]/.test(input.password)||!/\d/.test(input.password))return "Şifre en az 12 karakter, büyük/küçük harf ve sayı içermelidir.";
  if(input.password!==input.passwordConfirm)return "Şifreler eşleşmiyor.";
  return null;
}
